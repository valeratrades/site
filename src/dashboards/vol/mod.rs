#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
use v_utils::NowThen;

#[cfg(feature = "ssr")]
use crate::{conf::Settings, utils::Mock};

#[island]
pub fn VolView() -> impl IntoView {
	let duration = std::time::Duration::from_hours(24);
	let vol_resource = Resource::new(move || (), move |_| async move { try_pull(duration).await });

	div().child(Suspense(SuspenseProps {
		fallback: { || pre().child("Loading Vol Block...") }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || match vol_resource.get() {
				Some(Ok(vol_data)) => (pre().child(vol_data.to_string()),).into_any(),
				Some(Err(e)) => (pre().child(format!("Error loading Vol data: {e}")),).into_any(),
				None => (pre().child("Loading Vol Block..."),).into_any(),
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
