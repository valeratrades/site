#[cfg(feature = "ssr")]
pub mod _core;
pub mod cme;
pub mod deck;
pub mod fng;
pub mod lsr;
pub mod market_structure;
pub mod vol;
use leptos::{html::*, prelude::*};
use leptos_meta::{Title, TitleProps};
use leptos_routable::prelude::*;
use leptos_router::{
	components::{A, AProps, Outlet},
	hooks::use_location,
};

use crate::app::AppRoutes;

#[derive(Routable)]
#[routes(transition = false)]
pub enum Routes {
	#[route(path = "/")]
	Home,

	#[fallback]
	#[route(path = "/404")]
	NotFound,
}
// boilerplate {{{
#[component]
pub fn DashboardsView() -> impl IntoView {
	Outlet()
}
/// Animated loading indicator that blinks "..." on and off.
/// Uses CSS animation so it works immediately from SSR without waiting for hydration.
#[component]
pub fn LoadingIndicator(label: String) -> impl IntoView {
	pre().child((format!("Loading {label}"), span().class("loading-dots").child("...")))
}

/// Reads live fetch progress registered under `name`.
#[server]
async fn dashboard_progress(name: String) -> Result<Option<String>, ServerFnError> {
	Ok(_core::progress_of(&name))
}

/// Loading indicator that polls [`dashboard_progress`] (~500ms) and shows `X/Y` while a fetch runs.
#[component]
pub fn LoadingWithProgress(label: String, name: String) -> impl IntoView {
	let progress = RwSignal::new(None::<String>);
	#[cfg(feature = "hydrate")]
	{
		use gloo_timers::callback::Interval;
		use send_wrapper::SendWrapper;
		use wasm_bindgen_futures::spawn_local;

		let interval = SendWrapper::new(Interval::new(500, move || {
			let name = name.clone();
			spawn_local(async move {
				if let Ok(p) = dashboard_progress(name).await {
					progress.set(p);
				}
			});
		}));
		on_cleanup(move || drop(interval));
	}
	#[cfg(not(feature = "hydrate"))]
	drop(name);
	pre().child((
		move || match progress.get() {
			Some(p) => format!("Loading {label} — {p}"),
			None => format!("Loading {label}"),
		},
		span().class("loading-dots").child("..."),
	))
}
#[component]
fn NotFoundView() -> impl IntoView {
	let loc = use_location();
	section().class("p-4 text-center").child((
		h1().class("text-2xl font-bold").child("Dashboards Route Not Found"),
		p().child(move || format!("Path: {}", loc.pathname.get())),
		A(AProps {
			href: AppRoutes::Dashboards(Routes::Home),
			children: Box::new(|| view! { "Go to Dashboard Home" }.into_any()),
			target: None,
			exact: false,
			strict_trailing_slash: false,
			scroll: true,
		})
		.attr("class", "inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"),
	))
}
//,}}}

#[component]
fn HomeView() -> impl IntoView {
	section().child((
		Title(TitleProps {
			formatter: None,
			text: Some("Dashboards".into()),
		}),
		deck::DashboardDeck(),
	))
}
