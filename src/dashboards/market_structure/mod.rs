#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
#[cfg(feature = "hydrate")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "hydrate")]
#[wasm_bindgen(module = "/lwc_mount.js")]
extern "C" {
	async fn mount(el: web_sys::HtmlElement, src: &str);
}

#[component]
pub fn MarketStructureView() -> impl IntoView {
	let loading = RwSignal::new(true);
	#[cfg(feature = "hydrate")]
	{
		use gloo_timers::callback::Interval;
		use send_wrapper::SendWrapper;
		use wasm_bindgen_futures::spawn_local;

		let mount_chart = |on_done: Option<RwSignal<bool>>| {
			if let Some(el) = document().get_element_by_id("ms-chart") {
				let el: web_sys::HtmlElement = el.dyn_into().unwrap();
				spawn_local(async move {
					mount(el, "/data/market_structure.json").await;
					if let Some(loading) = on_done {
						loading.set(false);
					}
				});
			}
		};
		Effect::new(move |_| mount_chart(Some(loading)));
		// shim reuses the chart + per-pair series, swapping data in place — no teardown, no loading overlay
		let interval = SendWrapper::new(Interval::new(30 * 60 * 1000, move || mount_chart(None)));
		on_cleanup(move || drop(interval));
	}

	div().style("height:100%;position:relative").child((div().id("ms-chart").style("height:100%"), move || {
		loading.get().then(|| {
			div()
				.class("absolute inset-0 flex items-center justify-center")
				.child(super::LoadingWithProgress(super::LoadingWithProgressProps {
					label: "MarketStructure".into(),
					name: "MarketStructureChart".into(),
				}))
		})
	}))
}

#[cfg(feature = "ssr")]
pub async fn market_structure_json_handler() -> axum::response::Response {
	use axum::response::IntoResponse;

	match crate::dashboards::_core::load::<data::MarketStructureChart>().await {
		Ok(chart) => axum::Json(chart).into_response(),
		Err(e) => {
			tracing::error!("Failed to build market structure: {e:?}");
			(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
		}
	}
}
