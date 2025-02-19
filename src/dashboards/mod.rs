mod lsr;

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_routable::prelude::*;
use leptos_router::{
	components::{A, Outlet},
	hooks::use_location,
};
use lsr::RenderedLsr;
use v_utils::prelude::*;

use crate::{AppRoutes, utils::Mock as _};

#[derive(Routable)]
#[routes(transition = false)]
pub enum Routes {
	#[route(path = "")]
	Home,

	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

#[component]
pub fn HomeView() -> impl IntoView {
	// should be a) a resource, b) in its own component, which then c) is only loaded if it's included into the main `dashboards` selection, skeleton of which should in turn be defined here.
	//let tf = "5m".into();
	//let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
	//let lsrs = tokio::runtime::Runtime::new().unwrap().block_on(async { lsr::get(tf, range).await }).unwrap();
	//lsrs.persist().unwrap();
	//let lsrs = SortedLsrs::load_mock().unwrap(); //dbg
	//let displayed_lsrs: Vec<String> = lsrs.iter().map(|lsr| lsr.display_short().unwrap()).collect();

	let rendered_lsrs: Vec<RenderedLsr> = vec![
		RenderedLsr::new(("DOT", "USDT").into(), "DOT: 96%".to_string()),
		RenderedLsr::new(("DOGE", "USDT").into(), "DOGE: 30%".to_string()),
		RenderedLsr::new(("SOL", "USDT").into(), "SOL: 20%".to_string()),
	]; //dbg
	debug!(?rendered_lsrs);

	let search_input = RwSignal::new(String::default());
	let selected_items = RwSignal::new(Vec::<RenderedLsr>::new());

	let filtered_items = Memo::new(move |_| {
		let search = search_input.read();
		if search.is_empty() { vec![] } else { fzf(&search, &rendered_lsrs) }
	});

	let handle_input = move |ev: web_sys::Event| {
		let new_input = event_target_value(&ev);
		*search_input.write() = new_input;
	};

	let handle_select = move |selected: RenderedLsr| {
		todo!();
	};

	view! {
		<section class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Dashboards Home"</h1>
			<p>"My Dashboards"</p>

			// Search form
			<form class="relative" on:submit=|ev| ev.prevent_default()>
				<input
					type="text"
					placeholder="Search Dashboards"
					prop:value=search_input
					on:input=handle_input
					class="w-full p-2 border rounded"
				/>

				// Dropdown results container
				<div class="absolute w-full mt-1 max-h-48 overflow-y-auto bg-white border rounded shadow-lg">
					<For
						each={move || filtered_items.get()}
						key=|item| item.clone()
						children=move |item: RenderedLsr| {
								view! {
									<div
										class="p-2 hover:bg-gray-100 cursor-pointer text-left"
										on:click=move |_| handle_select(item.clone())
									>
										{item.rend.clone()}
									</div>
								}
						}
					/>
				</div>
			</form>

			// Selected items display
			<div class="mt-4 space-y-2">
				<For
					each={move || selected_items.read().to_vec()}
					key=|item| item.clone()
					children=move |item: RenderedLsr| {
							view! {
								<div class="flex items-center justify-between p-2 bg-gray-50 rounded">
									<span>{item.rend.clone()}</span>
								</div>
							}
					}
				/>
			</div>

			<p class="text-sm font-mono mt-4">"Query String: " {move || use_location().search}</p>
		</section>
	}
}

fn fzf(s: &str, available: &[RenderedLsr]) -> Vec<RenderedLsr> {
	// For now, using simple substring matching
	available.iter().filter(|v| v.rend.to_lowercase().contains(&s.to_lowercase())).cloned().collect()
}

#[component]
pub fn NotFoundView() -> impl IntoView {
	//let loc = use_location();
	//view! {
	//	<section class="p-4 text-center">
	//		<h1 class="text-2xl font-bold">"Dashboards Route Not Found"</h1>
	//		<p>{move || format!("Path: {}", loc.pathname.get())}</p>
	//		<A
	//			href=AppRoutes::Dashboards(Routes::Home)
	//			attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
	//		>
	//			"Go to Dashboard Home"
	//		</A>
	//	</section>
	//}
	"404" //dbg
}

#[component]
pub fn DashboardsView() -> impl IntoView {
	view! { <Outlet /> }
}
