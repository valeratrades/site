#[cfg(feature = "ssr")]
mod data;
use leptos::{html::*, prelude::*};

use crate::conf::Settings;
#[cfg(feature = "ssr")]
use crate::utils::Mock;

#[component]
pub fn FngView() -> impl IntoView {
	let fng_resource = Resource::new(move || (), |_| async move { try_build().await.expect("dbg") });
	div().child(Suspense(SuspenseProps {
		fallback: { || pre().child("Loading Fng block...") }.into(),
		#[rustfmt::skip]
		children: ToChildren::to_children(move || IntoRender::into_render(move || {
			fng_resource.get().map(|ms| pre().child(ms.0))
		})),
	}))
}

#[server]
async fn try_build() -> Result<FngRendered, ServerFnError> {
	crate::try_load_mock!(data::Fng; .into());

	let fng = data::btc_fngs_hourly(1)
		.await
		.expect("TODO: proper error handling")
		.into_iter()
		.next()
		.expect("TODO: error mapping");
	fng.persist()?;
	Ok(fng.into())
}
#[cfg(feature = "ssr")]
impl Mock for data::Fng {}
#[derive(Clone, Debug, Default, derive_new::new, serde::Deserialize, serde::Serialize)]
pub struct FngRendered(String);
#[cfg(feature = "ssr")]
impl From<data::Fng> for FngRendered {
	fn from(fng: data::Fng) -> Self {
		Self(format!("BTC Fear and Greed (alternative.me): {}", fng.value))
	}
}
