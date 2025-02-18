#![feature(duration_constructors)]
#![feature(stmt_expr_attributes)]
#![allow(non_snake_case)]

pub(crate) mod dashboards;
pub(crate) mod utils;

use dashboards::DashboardsView;
use leptos::prelude::*;
use leptos_meta::{Html, Meta, MetaTags, Title};
use leptos_routable::prelude::{combine_paths, Routable};
use leptos_router::components::{Router, A};

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	console_error_panic_hook::set_once();
	_ = console_log::init_with_level(tracing::log::Level::Debug); //Q: what is this, do I need it?
	leptos::mount::hydrate_body(App);
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<AutoReload options=options.clone() />
				<HydrationScripts options />
				<MetaTags />
			</head>
			<body>
				<App />
			</body>
		</html>
	}
}

#[component]
pub fn App() -> impl IntoView {
	leptos_meta::provide_meta_context();
	view! {
		<Html attr:lang="en" attr:dir="ltr" />
		<Title text="My sity-site" />
		<Meta charset="UTF-8" />
		<Meta name="viewport" content="width=device-width, initial-scale=1.0" />
		<main class="min-h-screen">
			<Router>
				<nav class="flex space-x-4 p-4 bg-gray-900 text-white">
					<A href=AppRoutes::Home attr:class="text-white px-3 py-1 bg-green-600 rounded">
						"Home"
					</A>
					<A
						href=AppRoutes::Dashboards(dashboards::Routes::Home)
						attr:class="text-white px-3 py-1 bg-blue-600 rounded"
					>
						"Dashboards"
					</A>
				</nav>
				{move || AppRoutes::routes()}
			</Router>
		</main>
	}
}

#[derive(Routable)]
#[routes(view_prefix = "", view_suffix = "View", transition = false)]
pub enum AppRoutes {
	#[route(path = "/")]
	Home,
	#[parent_route(path = "/dashboards", ssr = "::leptos_router::SsrMode::default()")]
	Dashboards(dashboards::Routes),
	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

#[component]
pub fn HomeView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Welcome Home!"</h1>
			<p>"Explore the site using the navigation links below."</p>
		</div>
	}
}

#[component]
pub fn NotFoundView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h1 class="text-2xl font-bold">"404: Not Found"</h1>
			<p>"Sorry, we can't find that page."</p>
			<A
				href=AppRoutes::Home
				attr:class="inline-block px-4 py-2 bg-green-500 text-white rounded mt-4"
			>
				"Go Home"
			</A>
		</div>
	}
}
