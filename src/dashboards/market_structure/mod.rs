#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};

#[component]
pub fn MarketStructureView() -> impl IntoView {
	let loading = RwSignal::new(true);
	let stale = RwSignal::new(None::<String>);
	#[cfg(feature = "hydrate")]
	{
		use gloo_timers::callback::Interval;
		use send_wrapper::SendWrapper;
		use wasm_bindgen::JsCast;
		use wasm_bindgen_futures::spawn_local;

		// staleness lives inside the payload; only Rust can read it back to drive the reactive banner,
		// so the fetch stays here rather than in the draw module (the shim returns null on success).
		#[derive(serde::Deserialize)]
		struct StaleField {
			stale: Option<String>,
		}

		let mount_chart = move |on_done: Option<RwSignal<bool>>| {
			if let Some(el) = document().get_element_by_id("ms-chart") {
				let el: web_sys::HtmlElement = el.dyn_into().unwrap();
				spawn_local(async move {
					match gloo_net::http::Request::get("/data/market_structure.json").send().await {
						Ok(resp) if !resp.ok() => {
							let body = resp.text().await.unwrap_or_default();
							stale.set(Some(format!("⚠ Exchange unavailable — {}", body.trim())));
						}
						Ok(resp) => match resp.text().await {
							Ok(body) => {
								// a malformed payload leaves banner None — the mount below surfaces its own chart-side error
								let banner = serde_json::from_str::<StaleField>(&body).ok().and_then(|s| s.stale);
								// a chart-side error overrides the server's staleness note
								stale.set(v_utils::lwc::mount(el, "/lwc_draw.js", &body, "null").await.or(banner));
							}
							Err(e) => stale.set(Some(format!("⚠ Exchange unavailable — {e}"))),
						},
						Err(e) => stale.set(Some(format!("⚠ Exchange unavailable — {e}"))),
					}
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

	div().style("height:100%;display:flex;flex-direction:column").child((
		move || {
			stale.get().map(|msg| {
				div()
					.class("shrink-0 bg-amber-500/15 text-amber-200 text-[11px] leading-snug px-2 py-1 border-b border-amber-500/30")
					.child(msg)
			})
		},
		div()
			.style("flex:1 1 auto;min-height:0;position:relative")
			.child((div().id("ms-chart").style("height:100%"), move || {
				loading.get().then(|| {
					div()
						.class("absolute inset-0 flex items-center justify-center")
						.child(super::LoadingWithProgress(super::LoadingWithProgressProps {
							label: "MarketStructure".into(),
							name: "MarketStructureChart".into(),
						}))
				})
			})),
	))
}

#[cfg(feature = "ssr")]
pub async fn market_structure_json_handler() -> axum::response::Response {
	use axum::response::IntoResponse;

	match crate::dashboards::_core::load::<data::MarketStructureChart>().await {
		Ok(loaded) => {
			#[derive(serde::Serialize)]
			struct Resp {
				#[serde(flatten)]
				chart: data::MarketStructureChart,
				#[serde(skip_serializing_if = "Option::is_none")]
				stale: Option<String>,
			}
			let stale = loaded
				.stale
				.map(|s| format!("⚠ Exchange rate-limited — data from {} may be outdated. {}", s.fetched_at.strftime("%Y-%m-%d %H:%M UTC"), s.error));
			axum::Json(Resp { chart: loaded.data, stale }).into_response()
		}
		Err(e) => {
			tracing::error!("Failed to build market structure: {e:?}");
			(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
		}
	}
}
