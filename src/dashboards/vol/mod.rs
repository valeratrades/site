#[cfg(feature = "ssr")]
mod data;

use leptos::{html::*, prelude::*};
use v_utils::NowThen;

use crate::conf::Settings;
#[cfg(feature = "ssr")]
use crate::utils::Mock;

#[component]
pub fn VolView() -> impl IntoView {
	let duration = std::time::Duration::from_hours(24);
	let vol_resource = Resource::new(move || (), move |_| async move { try_pull(duration).await.expect("dbg") });

	div().child(Suspense(SuspenseProps {
		fallback: { || pre().child("Loading Vol Block...") }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || {
				let vol_resource_clone = vol_resource.clone();
				vol_resource_clone.get().map(|v| pre().child(v.to_string()))
			})
		}),
	}))
}

#[server]
async fn try_pull(duration: std::time::Duration) -> Result<VolData, ServerFnError> {
	crate::try_load_mock!(VolData);

	let (vix, bvol) = tokio::try_join!(data::vix(duration), data::bvol(duration)).expect("TODO: conversion eyre -> SererFnError");
	let data = VolData::new(vix, bvol);

	data.persist()?;
	Ok(data)
}
#[cfg(feature = "ssr")]
impl Mock for VolData {}
//TODO!!!!: NowThen it
#[derive(Clone, Debug, Default, derive_new::new, serde::Deserialize, serde::Serialize)]
pub struct VolData {
	vix: NowThen,
	bvol: NowThen,
}
impl std::fmt::Display for VolData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "VIX: {}; BVOL: {}", self.vix, self.bvol)
	}
}
