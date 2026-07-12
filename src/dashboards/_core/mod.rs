//! Standardizes polled data sources: persist every poll to `$XDG_DATA_HOME`, and
//! don't hit upstream again until the persisted copy is older than [`SourceData::decay_horizon`].
use std::{future::Future, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::eyre::Result;
use serde::{Serialize, de::DeserializeOwned};
use v_utils::trades::Timeframe;

pub trait SourceData: Sized + Serialize + DeserializeOwned + Send {
	/// Both the poll frequency and the age past which a persisted copy is stale.
	/// Under normal load the persisted copy is practically always fresh.
	fn decay_horizon() -> Timeframe;
	fn fetch() -> impl Future<Output = Result<Self>> + Send;

	fn name() -> &'static str {
		let p = std::any::type_name::<Self>();
		p.rsplit("::").next().unwrap_or(p)
	}
}

/// Serve the persisted copy while fresh (or always, under mock); otherwise repoll and persist.
pub async fn load<T: SourceData>() -> Result<T> {
	if let Some(cached) = read::<T>() {
		let age = Utc::now().signed_duration_since(cached.fetched_at);
		let stale = age.to_std().map_or(true, |a| a >= T::decay_horizon().duration());
		if mock_enabled() || !stale {
			tracing::debug!("serving persisted {} (age {age})", T::name());
			return Ok(cached.data);
		}
		tracing::debug!("{} stale (age {age}), repolling", T::name());
	}
	let data = T::fetch().await?;
	write(&data)?;
	Ok(data)
}

fn mock_enabled() -> bool {
	leptos::prelude::use_context::<crate::config::LiveSettings>()
		.expect("LiveSettings in context")
		.config()
		.expect("config loads")
		.mock()
}

fn path<T: SourceData>() -> PathBuf {
	v_utils::xdg_data_dir!("dashboards").join(format!("{}.json", T::name()))
}

#[derive(serde::Deserialize)]
struct Cached<T> {
	fetched_at: DateTime<Utc>,
	data: T,
}
#[derive(Serialize)]
struct CachedRef<'a, T> {
	fetched_at: DateTime<Utc>,
	data: &'a T,
}

fn read<T: SourceData>() -> Option<Cached<T>> {
	let p = path::<T>();
	match std::fs::read_to_string(&p) {
		Ok(s) => match serde_json::from_str(&s) {
			Ok(c) => Some(c),
			Err(e) => {
				tracing::warn!("corrupt cache {}: {e}; refetching", p.display());
				None
			}
		},
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
		Err(e) => {
			tracing::warn!("failed reading cache {}: {e}; refetching", p.display());
			None
		}
	}
}

fn write<T: SourceData>(data: &T) -> Result<()> {
	let record = CachedRef { fetched_at: Utc::now(), data };
	let p = path::<T>();
	std::fs::write(&p, serde_json::to_string_pretty(&record)?)?;
	tracing::info!("persisted {}", T::name());
	Ok(())
}
