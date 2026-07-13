#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
use v_utils::NowThen;

use super::{LoadingIndicator, LoadingIndicatorProps};

#[component]
pub fn VolView() -> impl IntoView {
	let trigger = RwSignal::new(());
	let vol_resource = Resource::new(move || trigger.get(), move |_| async move { try_pull().await });

	// Set up retry interval - retry every 1 minute on error
	#[cfg(not(feature = "ssr"))]
	{
		Effect::new(move || {
			if let Some(Err(_)) = vol_resource.get() {
				set_timeout(
					move || {
						trigger.update(|_| ());
					},
					std::time::Duration::from_secs(60),
				);
			}
		});
	}

	div().class("panel-center").child(Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "Vol".into() }) }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || match vol_resource.get() {
				Some(Ok(vol_data)) => (pre().child(vol_data.to_string()),).into_any(),
				Some(Err(e)) => (pre().child(format!("Error loading Vol data: {e} (retrying...)")),).into_any(),
				None => (LoadingIndicator(LoadingIndicatorProps { label: "Vol".into() }),).into_any(),
			})
		}),
	}))
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, derive_new::new)]
pub struct VolData {
	vix: NowThen,
	bvol: NowThen,
}
#[server]
async fn try_pull() -> Result<VolData, ServerFnError> {
	super::_core::load::<VolData>().await.map(|l| l.data).map_err(|e| {
		tracing::error!("Failed to load volatility data: {e:?}");
		ServerFnError::new(format!("Failed to load volatility data: {e}"))
	})
}
#[cfg(feature = "ssr")]
impl super::_core::SourceData for VolData {
	fn decay_horizon() -> v_utils::trades::Timeframe {
		"1h".into()
	}

	async fn fetch() -> color_eyre::eyre::Result<Self> {
		let duration = std::time::Duration::from_hours(24);
		let (vix, bvol) = tokio::try_join!(data::vix(duration), data::bvol(duration))?;
		Ok(VolData::new(vix, bvol))
	}
}
//TODO!!!!: NowThen it
impl std::fmt::Display for VolData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "VIX: {}; BVOL: {}", self.vix, self.bvol)
	}
}
