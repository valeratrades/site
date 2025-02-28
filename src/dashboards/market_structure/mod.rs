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
