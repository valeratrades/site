#[cfg(feature = "ssr")]
mod data;

use leptos::{
	control_flow::{For, ForEnumerate, ForEnumerateProps},
	ev,
	html::*,
	prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::conf::Settings;
#[cfg(feature = "ssr")]
use crate::utils::Mock;

#[component]
pub fn MS() -> impl IntoView {
	//TODO: refresh
	view! {
		<Suspense fallback=move || {
				pre().child("Loading...")
		}>
			{move || Suspend::new(async move {
					let ms = request_market_structure().await.expect("TODO: handle error");
					pre().child(ms.0);
			})}
		</Suspense>
	}
}
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
//   // Create the view
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
async fn request_market_structure() -> Result<MarketStructure, ServerFnError> {
	crate::try_load_mock!(MarketStructure);

	let tf = "5m".into();
	let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
	let plot = data::try_build(range, tf, "Binance/Futures".into()).await.expect("TODO: correct mapping to ServerFnError");
	Ok(MarketStructure(plot.to_html()))
}
