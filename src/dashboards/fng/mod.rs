#[cfg(feature = "ssr")]
mod data;
use leptos::{html::*, prelude::*};

use super::{LoadingIndicator, LoadingIndicatorProps};

#[component]
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

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, derive_new::new)]
pub struct FngRendered(String);
#[server]
async fn try_build() -> Result<FngRendered, ServerFnError> {
	let fng = super::_core::load::<data::Fng>().await.map_err(|e| {
		tracing::error!("Failed to load Fear & Greed Index: {e:?}");
		ServerFnError::new(format!("Failed to load Fear & Greed Index: {e}"))
	})?;
	Ok(fng.into())
}
#[cfg(feature = "ssr")]
impl super::_core::SourceData for data::Fng {
	fn decay_horizon() -> v_utils::trades::Timeframe {
		"1h".into()
	}

	async fn fetch() -> color_eyre::eyre::Result<Self> {
		data::btc_fngs_hourly(1)
			.await?
			.into_iter()
			.next()
			.ok_or_else(|| color_eyre::eyre::eyre!("Fear & Greed Index response was empty"))
	}
}
#[cfg(feature = "ssr")]
impl From<data::Fng> for FngRendered {
	fn from(fng: data::Fng) -> Self {
		Self(format!("BTC Fear and Greed (alternative.me): {}", fng.value))
	}
}
