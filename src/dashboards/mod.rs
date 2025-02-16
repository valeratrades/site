mod lsr;

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_routable::prelude::*;
use leptos_router::{
	components::{Outlet, A},
	hooks::use_location,
};
//use lsrs::SortedLsrs;
use v_utils::prelude::*;

use crate::{utils::Mock as _, AppRoutes};

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
	let displayed_lsrs = vec![
		"BTC".to_string(),
		"ETH".to_string(),
		"ADA".to_string(),
		"DOT".to_string(),
		"DOGE".to_string(),
		"SOL".to_string(),
		"LUNA".to_string(),
		"AVAX".to_string(),
		"UNI".to_string(),
		"LINK".to_string(),
	]; //dbg
	//DO: here we should join the world of ssr and csr, so the Vec should contain type without any reference to v_exchanges.

	debug!(?displayed_lsrs);

	let (search_value, set_search_value) = signal(String::new());
	let (selected_items, set_selected_items) = signal(Vec::<String>::new());

	// Create a derived signal for filtered items
	let filtered_items = Memo::new(move |_| {
		let search = search_value.get();
		if search.is_empty() {
			vec![]
		} else {
			fzf(&search, &displayed_lsrs)
		}
	});

	let handle_input = move |ev: web_sys::Event| {
		let input = event_target_value(&ev);
		set_search_value.set(input);
	};

	let handle_select = move |item: String| {
		set_selected_items.update(|items| {
			if !items.contains(&item) {
				items.push(item);
			}
		});
		set_search_value.set(String::new());
	};

	let handle_remove = move |item: String| {
		set_selected_items.update(|items| {
			items.retain(|x| x != &item);
		});
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
					prop:value=search_value
					on:input=handle_input
					class="w-full p-2 border rounded"
				/>

				// Dropdown results container
				{move || {
						let items = filtered_items.get();
						view! {
							// if !items.is_empty() {
							<div class="absolute w-full mt-1 max-h-48 overflow-y-auto bg-white border rounded shadow-lg">
								<For
									each=move || filtered_items.get()
									key=|item| item.clone()
									children=move |item: String| {
											view! {
												<div
													class="p-2 hover:bg-gray-100 cursor-pointer text-left"
													on:click=move |_| handle_select(item.clone())
												>
													{item.clone()}
												</div>
											}
									}
								/>
							</div>
						}
								.into_view()
				}}
			</form>

			// Selected items display
			<div class="mt-4 space-y-2">
				<For
					each=move || selected_items.get()
					key=|item| item.clone()
					children=move |item: String| {
							view! {
								<div class="flex items-center justify-between p-2 bg-gray-50 rounded">
									<span>{item.clone()}</span>
									<button class="text-red-500 hover:text-red-700" on:click=move |_| handle_remove(item.clone())>
										"×"
									</button>
								</div>
							}
					}
				/>
			</div>

			<p class="text-sm font-mono mt-4">"Query String: " {move || use_location().search}</p>
		</section>
	}
}

fn fzf(s: &str, available: &[String]) -> Vec<String> {
	// For now, using simple substring matching
	available.iter().filter(|item| item.to_lowercase().contains(&s.to_lowercase())).cloned().collect()
}

#[component]
pub fn NotFoundView() -> impl IntoView {
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

#[component]
pub fn DashboardsView() -> impl IntoView {
	view! { <Outlet /> }
}
