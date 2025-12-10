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
use web_sys::wasm_bindgen::JsValue;

use super::{LoadingIndicator, LoadingIndicatorProps};
#[cfg(feature = "ssr")]
use crate::utils::Mock;

/// Main wrapper component that fetches LSR data and passes it to both search and display
#[component]
pub fn LsrView() -> impl IntoView {
	let trigger = RwSignal::new(());
	let lsrs_resource = Resource::new(move || trigger.get(), |_| async move { build_lsrs().await });

	// Set up interval to refresh data every 5 minutes
	#[cfg(feature = "ssr")]
	{
		use std::time::Duration;
		tokio::spawn(async move {
			loop {
				tokio::time::sleep(Duration::from_secs(5 * 60)).await;
				trigger.update(|_| ());
			}
		});
	}

	// Set up retry interval - retry every 1 minute on error (client-side)
	#[cfg(not(feature = "ssr"))]
	{
		Effect::new(move || {
			if let Some(Err(_)) = lsrs_resource.get() {
				set_timeout(
					move || {
						trigger.update(|_| ());
					},
					std::time::Duration::from_secs(60),
				);
			}
		});
	}

	section().class("p-4 text-center").child(Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "LSR".into() }) }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || match lsrs_resource.get() {
				Some(Ok(rendered_lsrs)) => {
					let outliers = rendered_lsrs.outliers.clone();
					let lsrs_vec = rendered_lsrs.v.clone();
					(div().child((
						pre().inner_html(outliers),
						LsrSearchAndDisplayIsland(LsrSearchAndDisplayIslandProps { rendered_lsrs: lsrs_vec.clone() }),
					)),)
						.into_any()
				}
				Some(Err(e)) => (pre().child(format!("Error loading Lsrs: {e} (retrying...)")), ().into_any()).into_any(),
				None => (LoadingIndicator(LoadingIndicatorProps { label: "LSR".into() }), ().into_any()).into_any(),
			})
		}),
	}))
}

#[island]
pub fn LsrSearchAndDisplayIsland(rendered_lsrs: Vec<RenderedLsr>) -> impl IntoView {
	let rendered_lsrs_memo = Memo::new(move |_| rendered_lsrs.clone());

	// Signal to track selected pairs from URL
	let selected_pairs = RwSignal::new(Vec::<Pair>::new());

	// Read URL params on mount and update selected_pairs
	#[cfg(not(feature = "ssr"))]
	{
		if let Some(window) = web_sys::window() {
			if let Ok(search) = window.location().search() {
				if let Some(lsrs_start) = search.find("lsrs=") {
					let after_lsrs = &search[lsrs_start + 5..];
					let end = after_lsrs.find('&').unwrap_or(after_lsrs.len());
					let lsrs_str = &after_lsrs[..end];
					let pairs: Vec<Pair> = lsrs_str.split(',').filter_map(|s| s.trim().parse::<Pair>().ok()).collect();
					selected_pairs.set(pairs);
				}
			}
		}
	}

	(
		LsrSearch(LsrSearchProps {
			rendered_lsrs: rendered_lsrs_memo,
			selected_pairs,
		}),
		LsrDisplay(LsrDisplayProps {
			rendered_lsrs: rendered_lsrs_memo,
			selected_pairs,
		}),
	)
}

#[component]
fn LsrDisplay(rendered_lsrs: Memo<Vec<RenderedLsr>>, selected_pairs: RwSignal<Vec<Pair>>) -> impl IntoView {
	let selected_lsrs = Memo::new(move |_| {
		let pairs = selected_pairs.get();
		let all_lsrs = rendered_lsrs.get();

		pairs.into_iter().filter_map(|pair| all_lsrs.iter().find(|lsr| lsr.pair == pair).cloned()).collect::<Vec<_>>()
	});

	div().class("mt-4 space-y-2").child(For(ForProps {
		each: move || selected_lsrs.get(),
		key: |item| item.clone(),
		children: move |item: RenderedLsr| div().class("flex items-center justify-between p-2 bg-gray-50 rounded").child(span().child(item.rend.clone())),
	}))
}

#[component]
fn LsrSearch(rendered_lsrs: Memo<Vec<RenderedLsr>>, selected_pairs: RwSignal<Vec<Pair>>) -> impl IntoView {
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

	let handle_select_click = StoredValue::new(move |item: RenderedLsr| {
		// Add to selected pairs
		selected_pairs.update(|pairs| {
			if !pairs.contains(&item.pair) {
				pairs.push(item.pair);
			}
		});

		// Update URL to reflect selection
		if let Some(window) = web_sys::window() {
			let pairs = selected_pairs.get();
			let lsrs_param = pairs.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
			if let Ok(history) = window.history() {
				let new_url = format!("/dashboards?lsrs={}", lsrs_param);
				let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&new_url));
			}
		}

		*search_input.write() = String::default();
	});

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
						handle_select_click.with_value(|f| f(selected_item));
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
							move |_| handle_select_click.with_value(|f| f(item_clone.clone())) //wtf, why must I clone twice?
						})
						.child(item.rend.clone())
				},
			})),
	))
}

#[server]
async fn build_lsrs() -> Result<RenderedLsrs, ServerFnError> {
	crate::try_load_mock!(data::SortedLsrs; .into());

	let tf = "5m".into();
	let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
	let lsrs = match data::get(tf, range).await {
		Ok(lsrs) => lsrs,
		Err(e) => {
			tracing::error!("Failed to fetch LSR data: {e:?}");
			return Err(ServerFnError::new(format!("Failed to fetch LSR data: {e}")));
		}
	};
	lsrs.persist()?;
	Ok(lsrs.into())
}
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, derive_new::new)]
pub struct RenderedLsr {
	pub pair: Pair,
	pub rend: String,
}
#[derive(Clone, Debug, Default, derive_more::Deref, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
