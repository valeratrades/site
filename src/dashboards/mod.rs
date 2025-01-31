use leptos::prelude::*;
use leptos_routable::prelude::*;
use leptos_router::{
	components::{A, Outlet},
	hooks::use_location,
};

use crate::AppRoutes;

#[derive(Routable)]
#[routes(transition = false)]
pub enum DashboardsRoutes {
	#[route(path = "")]
	DashboardsHome,

	#[route(path = "/settings")]
	DashboardsSettings,

	#[route(path = "/analytics")]
	DashboardsAnalytics,

	#[fallback]
	#[route(path = "/404")]
	DashboardsNotFound,
}

#[component]
pub fn DashboardsHomeView() -> impl IntoView {
	view! {
		<section class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Dashboards Home"</h1>
			<p>"Welcome to Dashboards!"</p>
			<A
				href=AppRoutes::Dashboards(DashboardsRoutes::DashboardsSettings)
				attr:class="inline-block px-4 py-2 bg-blue-600 text-white rounded mt-2"
			>
				"Go to Settings"
			</A>
			<A
				href=AppRoutes::Dashboards(DashboardsRoutes::DashboardsAnalytics)
				attr:class="inline-block px-4 py-2 bg-blue-600 text-white rounded mt-2 ml-2"
			>
				"Go to Analytics"
			</A>
		</section>
	}
}

#[component]
pub fn DashboardsSettingsView() -> impl IntoView {
	view! {
		<section class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Dashboards Settings"</h1>
			<p>"Configure dashboards' settings here."</p>
			<A
				href=AppRoutes::Dashboards(DashboardsRoutes::DashboardsHome)
				attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
			>
				"Back Home"
			</A>
		</section>
	}
}

#[component]
pub fn DashboardsAnalyticsView() -> impl IntoView {
	view! {
		<section class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Dashboards Analytics"</h1>
			<p>"Analytics overview."</p>
			<A
				href=AppRoutes::Dashboards(DashboardsRoutes::DashboardsHome)
				attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
			>
				"Back Home"
			</A>
		</section>
	}
}

#[component]
pub fn DashboardsNotFoundView() -> impl IntoView {
	let loc = use_location();
	view! {
		<section class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Dashboards Route Not Found"</h1>
			<p>{move || format!("Path: {}", loc.pathname.get())}</p>
			<A
				href=AppRoutes::Dashboards(DashboardsRoutes::DashboardsHome)
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
