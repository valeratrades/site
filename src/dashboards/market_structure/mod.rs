#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
#[cfg(feature = "ssr")]
use v_exchanges::{ExchangeName, Instrument};

use crate::conf::Settings;
#[cfg(feature = "ssr")]
use crate::utils::Mock;

#[island]
pub fn MarketStructureView() -> impl IntoView {
	//TODO: refresh (am I certain I care for it?)
	//Q: potentially switch to LocalResource
	//TODO: pass <leptos_meta::Script src="https://cdn.plot.ly/plotly-latest.min.js"></Script> from here, so that we don't unconditionally import it during init
	//TODO: switch to convention of displaying `Loading...` above already loaded data (if any) on reload, instead of replacing it.
	let ms_resource = Resource::new(move || (), |_| async move { request_market_structure().await.expect("dbg") });
	#[rustfmt::skip]
	Suspense(SuspenseProps {
		fallback: { || pre().child("Loading MarketStructure...") }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || {
				ms_resource.get().map(|ms| {
					div().inner_html(ms.0)
				})
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
	let plot = data::try_build(range, tf, ExchangeName::Binance, Instrument::Perp)
		.await
		.expect("TODO: correct mapping to ServerFnError");
	let ms = MarketStructure(plot.to_inline_html(None)); //Q: not sure if provision of div id is necessary
	ms.persist()?;
	Ok(ms)
}
