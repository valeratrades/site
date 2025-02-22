mod lsr;

use leptos::{
	control_flow::{For, ForEnumerate, ForEnumerateProps},
	ev,
	html::*,
	prelude::*,
};
use leptos_meta::{Title, TitleProps};
use leptos_routable::prelude::*;
use leptos_router::{
	components::{A, Outlet},
	hooks::use_location,
};
use lsr::RenderedLsr;

#[cfg(feature = "ssr")]
use crate::utils::Mock as _;
use crate::{app::AppRoutes, conf::Settings};

#[derive(Routable)]
#[routes(transition = false)]
pub enum Routes {
	#[route(path = "")]
	Home,

	#[fallback]
	#[route(path = "/404")]
	NotFound,
}
// boilerplate {{{
#[component]
pub fn DashboardsView() -> impl IntoView {
	view! { <Outlet /> }
}
#[component]
fn NotFoundView() -> impl IntoView {
	let loc = use_location();
	view! {
		<section class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Dashboards Route Not Found"</h1>
			<p>{move || format!("Path: {}", loc.pathname.get())}</p>
			<A
				href=AppRoutes::Dashboards(Routes::Home)
				attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
			>
				"Go to Dashboard Home"
			</A>
		</section>
	}
}
//,}}}

#[component]
fn HomeView() -> impl IntoView {
	#[rustfmt::skip]
	section().class("p-4 text-center").child((
		Title(TitleProps{
			formatter: None,
			text: Some("Dashboards".into()),
		}),
		p().child("My Dashboards"),
		Lsr(),
		p().class("text-sm font-mono mt-4")
			.child(( "Query String (TODO: handle):", move || use_location().search))
	))
}

#[component]
fn Lsr() -> impl IntoView {
	// should be a) a resource, b) in its own component, which then c) is only loaded if it's included into the main `dashboards` selection, skeleton of which should in turn be defined here.
	//let tf = "5m".into();
	//let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
	//let lsrs = tokio::runtime::Runtime::new().unwrap().block_on(async { lsr::get(tf, range).await }).unwrap();
	//lsrs.persist().unwrap();
	//let lsrs = SortedLsrs::load_mock().unwrap(); //dbg
	//let displayed_lsrs: Vec<String> = lsrs.iter().map(|lsr| lsr.display_short().unwrap()).collect();

	#[cfg(feature = "ssr")]
	let settings = use_context::<Settings>().unwrap_or(Settings { mock: true }); //dbg

	#[server]
	async fn build_lsrs(mock: bool) -> Result<Vec<RenderedLsr>, ServerFnError> {
		use lsr::ssr::SortedLsrs;
		//let v = vec![
		//	RenderedLsr::new(("DOT", "USDT").into(), "DOT: 96%".to_string()),
		//	RenderedLsr::new(("DOGE", "USDT").into(), "DOGE: 30%".to_string()),
		//	RenderedLsr::new(("SOL", "USDT").into(), "SOL: 20%".to_string()),
		//	RenderedLsr::new(("ADA", "USDT").into(), "ADA: 66%".to_string()),
		//]; //dbg
		let v = match mock {
			true => {
				//	vec![
				//	RenderedLsr::new(("DOT", "USDT").into(), "DOT: 96%".to_string()),
				//	RenderedLsr::new(("DOGE", "USDT").into(), "DOGE: 30%".to_string()),
				//	RenderedLsr::new(("SOL", "USDT").into(), "SOL: 20%".to_string()),
				//	RenderedLsr::new(("ADA", "USDT").into(), "ADA: 66%".to_string()),
				//]
				let lsrs = SortedLsrs::load_mock().unwrap(); //dbg
				lsrs.into()
			}
			false => {
				let tf = "5m".into();
				let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
				let lsrs = lsr::ssr::get(tf, range).await.expect("TODO: proper error handling");
				lsrs.persist().unwrap();
				lsrs.into()
			}
		};
		Ok(v)
	}

	let mock = true; //dbg
	let lsrs_resource = Resource::new(
		move || mock, // Dependency signal - rebuild if mock changes
		|mock_value| async move { build_lsrs(mock_value).await },
	);
	let rendered_lsrs_mem = Memo::new(move |_| {
		match lsrs_resource.get() {
			Some(Ok(lsrs)) => lsrs,
			_ => vec![], // Return empty vector if loading or error
		}
	});

	let selected_items = RwSignal::new(Vec::<RenderedLsr>::new());

	#[rustfmt::skip]
	section().class("p-4 text-center").child((
		LsrSearch(LsrSearchProps{
			rendered_lsrs: rendered_lsrs_mem,
			selected_items: selected_items.write_only(),
		}),
		// Selected items display
		div().class("mt-4 space-y-2")
			.child(
				For(ForProps {
					each: move || selected_items.read().to_vec(),
					key: |item| item.clone(),
					children: move |item: RenderedLsr| {
						div().class("flex items-center justify-between p-2 bg-gray-50 rounded")
							.child(
								span().child(item.rend.clone())
							)
					}
				})
			),
	))
}

#[component]
fn LsrSearch(rendered_lsrs: Memo<Vec<RenderedLsr>>, selected_items: WriteSignal<Vec<RenderedLsr>>) -> impl IntoView {
	let search_input = RwSignal::new(String::default());

	fn fzf(s: &str, available: &[RenderedLsr]) -> Vec<RenderedLsr> {
		// For now, using simple substring matching
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

	let focused_index = RwSignal::new(0); // -1 means no selection //TODO: make default to 0, then get rid of special-case handle_submit
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

	#[rustfmt::skip]
		form().class("inline-block")
			.child((
				input().class("p-2 border rounded")
					.attr("type", "text")
					.attr("placeholder", "Search Dashboards")
					.prop("value", search_input)
					.on(ev::input, handle_search_input)
					.on(ev::keydown, handle_key_nav),

				// Dropdown results container
				div()
					.class("relative inline-block z-50 overflow-y-auto") //Q: +"max-h-96" for overflow?
					.style(move || {
						if filtered_items.get().is_empty() {
							"display: none;"
						} else {
							"display: block;"
						}
					})
					.child(
						ForEnumerate(ForEnumerateProps {
							each: move || filtered_items.get(),
							key: |item| item.clone(),
							children: move |i: ReadSignal<usize>, item: RenderedLsr| {
								let is_focused = Memo::new(move |_| focused_index.get() == *i.read() as i32);

								//TEST
								static HOVER_BG: &str = "bg-gray-100";
								let bg = match is_focused() {
									true => HOVER_BG.to_owned(),
									false => format!("hover:{HOVER_BG}"),
								};
								//let bg = "hover:bg-gray-100"; //TODO: change bg (same as on hover) when `focused_index` matches `index`

								div()
									.class(format!("w-full cursor-pointer text-left z-50 {bg}"))
									.on(ev::click, {
										let item_clone = item.clone();
										move |_| handle_select_click(item_clone.clone()) //wtf, why must I clone twice?
									})
									.child(item.rend.clone())
							}
						}
						)
					),
			))
}
