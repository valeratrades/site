#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
use v_utils::NowThen;

use super::{LoadingIndicator, LoadingIndicatorProps};
#[cfg(feature = "ssr")]
use crate::{conf::Settings, utils::Mock};

#[island]
pub fn VolView() -> impl IntoView {
	let duration = std::time::Duration::from_hours(24);
	let trigger = RwSignal::new(());
	let vol_resource = Resource::new(move || trigger.get(), move |_| async move { try_pull(duration).await });

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

	div().child(Suspense(SuspenseProps {
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

#[server]
async fn try_pull(duration: std::time::Duration) -> Result<VolData, ServerFnError> {
	crate::try_load_mock!(VolData);

	let (vix, bvol) = match tokio::try_join!(data::vix(duration), data::bvol(duration)) {
		Ok(results) => results,
		Err(e) => {
			tracing::error!("Failed to fetch volatility data: {e:?}");
			return Err(ServerFnError::new(format!("Failed to fetch volatility data: {e}")));
		}
	};
	let data = VolData::new(vix, bvol);

	data.persist()?;
	Ok(data)
}
#[cfg(feature = "ssr")]
impl Mock for VolData {}
//TODO!!!!: NowThen it
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, derive_new::new)]
pub struct VolData {
	vix: NowThen,
	bvol: NowThen,
}
impl std::fmt::Display for VolData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "VIX: {}; BVOL: {}", self.vix, self.bvol)
	}
}
