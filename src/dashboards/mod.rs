use leptos::prelude::*;
use leptos_routable::prelude::*;
use leptos_router::{
	components::{Outlet, A},
	hooks::use_location,
};

use crate::AppRoutes;

#[derive(Routable)]
#[routes(transition = false)]
pub enum DashboardRoutes {
	#[route(path = "")]
	DashboardHome,

	#[route(path = "/settings")]
	DashboardSettings,

	#[route(path = "/analytics")]
	DashboardAnalytics,

	#[fallback]
	#[route(path = "/404")]
	DashboardNotFound,
}

#[component]
pub fn DashboardHomeView() -> impl IntoView {
	view! {
	  <section class="p-4 text-center">
		<h1 class="text-2xl font-bold">"Dashboard Home"</h1>
		<p>"Welcome to the Dashboard!"</p>
		<A
		  href=AppRoutes::Dashboard(DashboardRoutes::DashboardSettings)
		  attr:class="inline-block px-4 py-2 bg-blue-600 text-white rounded mt-2"
		>
		  "Go to Settings"
		</A>
		<A
		  href=AppRoutes::Dashboard(DashboardRoutes::DashboardAnalytics)
		  attr:class="inline-block px-4 py-2 bg-blue-600 text-white rounded mt-2 ml-2"
		>
		  "Go to Analytics"
		</A>
	  </section>
	}
}

#[component]
pub fn DashboardSettingsView() -> impl IntoView {
	view! {
	  <section class="p-4 text-center">
		<h1 class="text-2xl font-bold">"Dashboard Settings"</h1>
		<p>"Configure your dashboard settings here."</p>
		<A
		  href=AppRoutes::Dashboard(DashboardRoutes::DashboardHome)
		  attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
		>
		  "Back Home"
		</A>
	  </section>
	}
}

#[component]
pub fn DashboardAnalyticsView() -> impl IntoView {
	view! {
	  <section class="p-4 text-center">
		<h1 class="text-2xl font-bold">"Dashboard Analytics"</h1>
		<p>"Analytics overview."</p>
		<A
		  href=AppRoutes::Dashboard(DashboardRoutes::DashboardHome)
		  attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
		>
		  "Back Home"
		</A>
	  </section>
	}
}

#[component]
pub fn DashboardNotFoundView() -> impl IntoView {
	let loc = use_location();
	view! {
	  <section class="p-4 text-center">
		<h1 class="text-2xl font-bold">"Dashboard Route Not Found"</h1>
		<p>{move || format!("Path: {}", loc.pathname.get())}</p>
		<A
		  href=AppRoutes::Dashboard(DashboardRoutes::DashboardHome)
		  attr:class="inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"
		>
		  "Go to Dashboard Home"
		</A>
	  </section>
	}
}

#[component]
pub fn DashboardView() -> impl IntoView {
	view! { <Outlet /> }
}
