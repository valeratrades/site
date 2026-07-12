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

#[island]
pub fn MarketStructureView() -> impl IntoView {
	#[cfg(feature = "hydrate")]
	{
		use gloo_timers::callback::Interval;
		use send_wrapper::SendWrapper;
		use wasm_bindgen_futures::spawn_local;

		let render = move || {
			if let Some(el) = document().get_element_by_id("ms-chart") {
				let el: web_sys::HtmlElement = el.dyn_into().unwrap();
				spawn_local(async move {
					mount(el, "/data/market_structure.json").await;
				});
			}
		};
		Effect::new(move |_| render());
		// shim reuses the chart + per-pair series, swapping data in place — no teardown
		let interval = SendWrapper::new(Interval::new(30 * 60 * 1000, move || render()));
		on_cleanup(move || drop(interval));
	}

	div().id("ms-chart").style("height:70vh;position:relative")
}

#[cfg(feature = "ssr")]
pub async fn market_structure_json_handler() -> axum::response::Response {
	use axum::response::IntoResponse;
	use v_exchanges::{ExchangeName, Instrument};

	let tf = "5m".into();
	let range = (24 * 12 + 1).into(); // 24h, given `5m` tf

	match data::market_structure_json(range, tf, ExchangeName::Binance, Instrument::Perp).await {
		Ok(chart) => axum::Json(chart).into_response(),
		Err(e) => {
			tracing::error!("Failed to build market structure: {e:?}");
			(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
		}
	}
}
