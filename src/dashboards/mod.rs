pub mod cme;
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
	section().class("p-4 text-center").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Dashboards".into()),
		}),
		market_structure::MarketStructureView(),
		lsr::LsrView(),
		cme::CftcReportView(),
		vol::VolView(),
		fng::FngView(),
	))
}

/// Animated loading indicator that blinks "..." on and off.
/// Uses CSS animation so it works immediately from SSR without waiting for hydration.
#[component]
pub fn LoadingIndicator(label: String) -> impl IntoView {
	pre().child((format!("Loading {label}"), span().class("loading-dots").child("...")))
}
