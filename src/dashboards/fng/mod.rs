#[cfg(feature = "ssr")]
mod data;
use leptos::{html::*, prelude::*};

use super::{LoadingIndicator, LoadingIndicatorProps};
#[cfg(feature = "ssr")]
use crate::{config::Settings, utils::Mock};

#[island]
pub fn FngView() -> impl IntoView {
	let trigger = RwSignal::new(());
	let fng_resource = Resource::new(move || trigger.get(), |_| async move { try_build().await });

	// Set up retry interval - retry every 1 minute on error
	#[cfg(not(feature = "ssr"))]
	{
		Effect::new(move || {
			if let Some(Err(_)) = fng_resource.get() {
				set_timeout(
					move || {
						trigger.update(|_| ());
					},
					std::time::Duration::from_secs(60),
				);
			}
		});
	}

	div().child(Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "Fear & Greed".into() }) }.into(),
		#[rustfmt::skip]
		children: ToChildren::to_children(move || IntoRender::into_render(move || match fng_resource.get() {
			Some(Ok(fng_data)) => (pre().child(fng_data.0),).into_any(),
			Some(Err(e)) => (pre().child(format!("Error loading Fear & Greed: {e} (retrying...)")),).into_any(),
			None => (LoadingIndicator(LoadingIndicatorProps { label: "Fear & Greed".into() }),).into_any(),
		})),
	}))
}

#[server]
async fn try_build() -> Result<FngRendered, ServerFnError> {
	crate::try_load_mock!(data::Fng; .into());

	let fngs = data::btc_fngs_hourly(1).await.map_err(|e| {
		tracing::error!("Failed to fetch Fear & Greed Index: {e:?}");
		ServerFnError::new(format!("Failed to fetch Fear & Greed Index: {e}"))
	})?;

	let fng = fngs.into_iter().next().ok_or_else(|| {
		tracing::error!("Fear & Greed Index response was empty");
		ServerFnError::new("Fear & Greed Index response was empty")
	})?;

	fng.persist()?;
	Ok(fng.into())
}
#[cfg(feature = "ssr")]
impl Mock for data::Fng {}
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, derive_new::new)]
pub struct FngRendered(String);
#[cfg(feature = "ssr")]
impl From<data::Fng> for FngRendered {
	fn from(fng: data::Fng) -> Self {
		Self(format!("BTC Fear and Greed (alternative.me): {}", fng.value))
	}
}
