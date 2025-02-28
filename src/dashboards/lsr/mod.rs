#[cfg(feature = "ssr")]
mod data;
use leptos::{
	control_flow::{For, ForEnumerate, ForEnumerateProps},
	ev,
	html::*,
	prelude::*,
};
use serde::{Deserialize, Serialize};
use v_utils::trades::Pair;

use crate::conf::Settings;
#[cfg(feature = "ssr")]
use crate::utils::Mock;

#[island]
pub fn Lsr() -> impl IntoView {
	let lsrs_resource = Resource::new(move || (), |_| async move { build_lsrs().await });
	let rendered_lsrs = Memo::new(move |_| {
		match lsrs_resource.get() {
			Some(Ok(lsrs)) => lsrs.v,
			_ => vec![], // fallback
		}
	});

	//TODO: spawn to update every 15m

	let selected_items = RwSignal::new(Vec::<RenderedLsr>::new());

	section().class("p-4 text-center").child((
		div().child((
			Suspense(SuspenseProps {
				fallback: { || pre().child("Loading...") }.into(),
				children: ToChildren::to_children(move || {
					IntoRender::into_render(move || match lsrs_resource.get() {
						Some(Ok(l)) => pre().child(l.outliers),
						Some(Err(e)) => pre().child(format!("Error loading Lsrs: {e}")),
						None => pre().child("Loading...".to_owned()),
					})
				}),
			}),
			LsrSearch(LsrSearchProps {
				rendered_lsrs,
				selected_items: selected_items.write_only(),
			}),
		)),
		// Selected items display
		div().class("mt-4 space-y-2").child(For(ForProps {
			each: move || selected_items.read().to_vec(),
			key: |item| item.clone(),
			children: move |item: RenderedLsr| div().class("flex items-center justify-between p-2 bg-gray-50 rounded").child(span().child(item.rend.clone())),
		})),
	))
}

#[component]
fn LsrSearch(rendered_lsrs: Memo<Vec<RenderedLsr>>, selected_items: WriteSignal<Vec<RenderedLsr>>) -> impl IntoView {
	let search_input = RwSignal::new(String::default());

	///HACK: for now, using simple substring matching
	fn fzf(s: &str, available: &[RenderedLsr]) -> Vec<RenderedLsr> {
		available.iter().filter(|v| v.rend.to_lowercase().contains(&s.to_lowercase())).cloned().collect()
	}
	let filtered_items = Memo::new(move |_| {
		let search = search_input.read();
		if search.is_empty() { vec![] } else { fzf(&search, &rendered_lsrs.read()) }
	});

	let handle_search_input = move |ev: web_sys::Event| {
		let new_input = event_target_value(&ev);
		*search_input.write() = new_input;
	};

	let handle_select_click = move |item: RenderedLsr| {
		let selected = item.clone();
		selected_items.update(|items| {
			if !items.contains(&selected) {
				items.push(selected);
			}
		});

		*search_input.write() = String::default();
	};

	let focused_index = RwSignal::new(0);
	let handle_key_nav = move |ev: web_sys::KeyboardEvent| {
		if !filtered_items.get().is_empty() {
			let items_count = filtered_items.get().len();

			match ev.key().as_str() {
				"ArrowDown" => {
					ev.prevent_default();
					focused_index.update(|idx| {
						if *idx < (items_count as i32 - 1) {
							*idx += 1
						} else {
							*idx = 0 // Wrap around to first item
						}
					});
				}
				"ArrowUp" => {
					ev.prevent_default();
					focused_index.update(|idx| {
						if *idx > 0 {
							*idx -= 1
						} else {
							*idx = items_count as i32 - 1 // Wrap around to last item
						}
					});
				}
				"Enter" => {
					ev.prevent_default();
					let idx = focused_index.get();
					if idx >= 0 && (idx as usize) < items_count {
						let selected_item = filtered_items.get()[idx as usize].clone();
						handle_select_click(selected_item);
					}
					focused_index.set(0);
				}
				"Escape" => {
					ev.prevent_default();
					focused_index.set(0);
				}
				_ => {}
			}
		}
	};
	form().class("inline-block").child((
		input()
			.class("p-2 border rounded")
			.attr("type", "text")
			.attr("placeholder", "Search Dashboards")
			.prop("value", search_input)
			.on(ev::input, handle_search_input)
			.on(ev::keydown, handle_key_nav),
		// Dropdown results container
		div()
			.class("relative inline-block z-50 overflow-y-auto max-h-96")
			.style(move || if filtered_items.get().is_empty() { "display: none;" } else { "display: block;" })
			.child(ForEnumerate(ForEnumerateProps {
				each: move || filtered_items.get(),
				key: |item| item.clone(),
				children: move |i: ReadSignal<usize>, item: RenderedLsr| {
					let is_focused = move || focused_index.get() == *i.read() as i32;
					static HOVER_BG: &str = "bg-gray-100";

					div()
						.class(move || {
							let base_classes = "w-full cursor-pointer text-left";
							let on_focus = if is_focused() { HOVER_BG } else { "" };

							format!("{base_classes} hover:{HOVER_BG} {on_focus}") //TODO: fix hover bg ("hover:" tag is included to css, but nothing happens on hover)
						})
						.on(ev::click, {
							let item_clone = item.clone();
							move |_| handle_select_click(item_clone.clone()) //wtf, why must I clone twice?
						})
						.child(item.rend.clone())
				},
			})),
	))
}

#[server]
async fn build_lsrs() -> Result<RenderedLsrs, ServerFnError> {
	crate::try_load_mock! {data::SortedLsrs; .into()};

	let tf = "5m".into();
	let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
	let lsrs = data::get(tf, range).await.expect("TODO: proper error handling");
	lsrs.persist().unwrap();
	Ok(lsrs.into())
}
#[derive(Debug, Default, Clone, derive_new::new, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedLsr {
	pub pair: Pair,
	pub rend: String,
}
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::Deref)]
pub struct RenderedLsrs {
	#[deref]
	pub v: Vec<RenderedLsr>,
	pub outliers: String,
}
#[cfg(feature = "ssr")]
impl From<data::SortedLsrs> for RenderedLsrs {
	fn from(s: data::SortedLsrs) -> Self {
		Self {
			v: s.iter().map(|lsr| RenderedLsr::new(lsr.pair, lsr.display_short().unwrap())).collect(),
			outliers: s.display_outliers(10),
		}
	}
}
