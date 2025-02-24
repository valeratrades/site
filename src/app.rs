use leptos::{
	ev,
	html::{button, div, h1, p},
	prelude::*,
};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_routable::prelude::*;
use leptos_router::components::{A, Router};

use crate::dashboards::{self, DashboardsView};

pub fn shell(options: LeptosOptions) -> impl IntoView {
	view! {
		<!DOCTYPE html> 
		<html lang="en">
			<head>
				<meta charset="utf-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<AutoReload options=options.clone() />
				<HydrationScripts options islands=true />
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
	provide_meta_context();
	//TODO: with strum and derive_more, gen a nav bar at the top for all routes (ref: `nested` example in leptos-routable)
	let href = format!("/pkg/{}.css", env!("CARGO_PKG_NAME"));
	view! {
		<Stylesheet id="leptos" href=href />

		<Title text="My Site" />

		<main class="min-h-screen">
			<Router>{move || AppRoutes::routes()}</Router>
		</main>
	}
}

#[allow(dead_code)]
#[derive(Routable)]
#[routes(view_prefix = "", view_suffix = "View", transition = false)]
pub enum AppRoutes {
	#[route(path = "/")]
	Home,
	#[parent_route(path = "/dashboards")]
	Dashboards(dashboards::Routes),
	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

/// Renders the home page of your application.
#[component]
fn HomeView() -> impl IntoView {
	#[rustfmt::skip]
	div().child((
		h1().child("Welcome to Leptos!"),
		HomeButton(),
		p().class("bg-purple-500 text-white p-2 rounded m-2")
			.child("Tailwind check: this should have purple background and rounded corners") //dbg
	))
}

//dbg
#[island]
fn HomeButton() -> impl IntoView {
	let count = RwSignal::new(0);
	let on_click = move |_| *count.write() += 1;

	button().on(ev::click, on_click).child(move || format!("Click Me: {}", count.read()))
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
