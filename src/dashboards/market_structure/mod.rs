#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
#[cfg(feature = "ssr")]
use v_exchanges::{ExchangeName, Instrument};

use super::{LoadingIndicator, LoadingIndicatorProps};
#[cfg(feature = "ssr")]
use crate::{conf::Settings, utils::Mock};

#[island]
pub fn MarketStructureView() -> impl IntoView {
	//Q: potentially switch to LocalResource
	//TODO: pass <leptos_meta::Script src="https://cdn.plot.ly/plotly-latest.min.js"></Script> from here, so that we don't unconditionally import it during init
	//TODO: switch to convention of displaying `Loading...` above already loaded data (if any) on reload, instead of replacing it.

	let trigger = RwSignal::new(());
	let ms_resource = LocalResource::new(move || {
		trigger.get();
		async move { request_market_structure().await.expect("dbg") }
	});

	// Set up interval to refresh data every 30 minutes (client-side only)
	#[cfg(feature = "hydrate")]
	{
		use gloo_timers::callback::Interval;
		use send_wrapper::SendWrapper;

		let interval = SendWrapper::new(Interval::new(30 * 60 * 1000, move || {
			trigger.update(|_| ());
		}));
		on_cleanup(move || drop(interval));
	}

	#[rustfmt::skip]
	Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "MarketStructure".into() }) }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || {
				ms_resource.get().map(|ms| div().inner_html(ms.0))
			})
		}),
	})
}
//Q: not sure I actually need this refreshing at all, why not just drive it by manual refreshes?
//let (trigger, set_trigger) = create_signal(());
//
//   // Create the resource that depends on the trigger
//   let data = create_resource(
//       move || trigger(),  // This dependency means the resource will reload when trigger changes
//       move |_| get_data() // The async function to call
//   );
//
//   // Set up interval to refresh data every 15 minutes
//   use_interval(Duration::from_secs(15 * 60), move || {
//       set_trigger.update(|_| ());
//   });
//
//
//   view! {
//       <div>
//           {move || match data.get() {
//               None => view! { <p>"Loading..."</p> }.into_view(),
//               Some(result) => match result {
//                   Ok(data) => view! { <p>"Data: " {data}</p> }.into_view(),
//                   Err(e) => view! { <p>"Error: " {e.to_string()}</p> }.into_view(),
//               }
//           }}
//       </div>
//   }

/// Contains [Plot](plotly::Plot) represented as an html string (for csr compat)
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct MarketStructure(pub String);

#[cfg(feature = "ssr")]
impl Mock for MarketStructure {}
#[server]
pub async fn request_market_structure() -> Result<MarketStructure, ServerFnError> {
	crate::try_load_mock!(MarketStructure);

	let tf = "5m".into();
	let range = (24 * 12 + 1).into(); // 24h, given `5m` tf

	let result = data::try_build(range, tf, ExchangeName::Binance, Instrument::Perp).await;

	#[cfg(feature = "ssr")]
	fn render_error_html(error_msg: &str) -> String {
		use leptos::tachys::view::RenderHtml as _;

		div()
			.attr("style", "padding: 20px; background-color: #fee; border: 1px solid #c33; border-radius: 4px; color: #c33;")
			.child((
				h3().child("Error loading Market Structure"),
				p().child(error_msg.to_owned()),
				p().attr("style", "font-size: 0.9em; color: #666;").child("Check server logs for details."),
			))
			.to_html()
	}
	let html = match result {
		Ok(plot) => plot.to_inline_html(None),
		Err(e) => {
			tracing::error!("Failed to build market structure: {:?}", e);
			render_error_html(&e.to_string())
		}
	};

	let ms = MarketStructure(html);
	ms.persist()?;
	Ok(ms)
}
