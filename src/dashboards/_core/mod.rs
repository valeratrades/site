//! Standardizes polled data sources: persist every poll to `$XDG_DATA_HOME`, and
//! don't hit upstream again until the persisted copy is older than [`SourceData::decay_horizon`].
use std::{
	collections::HashMap,
	future::Future,
	path::PathBuf,
	sync::{LazyLock, Mutex},
};

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

	/// How an in-flight fetch's `(loaded, target)` is rendered to the client poller.
	fn fmt_progress(loaded: usize, target: usize) -> String {
		format!("{loaded}/{target}")
	}
}

/// Live per-source fetch progress, opted into by multi-pair sources inside their `fetch()`.
#[derive(Clone, Copy)]
pub struct Progress {
	name: &'static str,
}
impl Progress {
	pub fn of<T: SourceData>() -> Self {
		Self { name: T::name() }
	}

	pub fn set_target(&self, target: usize) {
		if let Some(e) = REGISTRY.lock().unwrap().get_mut(self.name) {
			e.target = target;
		}
	}

	pub fn inc(&self) {
		if let Some(e) = REGISTRY.lock().unwrap().get_mut(self.name) {
			e.loaded += 1;
		}
	}
}

/// Formatted progress of an in-flight fetch, or `None` when nothing is loading under `name`.
pub fn progress_of(name: &str) -> Option<String> {
	let reg = REGISTRY.lock().unwrap();
	reg.get(name).map(|e| (e.fmt)(e.loaded, e.target))
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
	REGISTRY.lock().unwrap().insert(
		T::name(),
		Entry {
			loaded: 0,
			target: 0,
			fmt: T::fmt_progress,
		},
	);
	let result = T::fetch().await;
	REGISTRY.lock().unwrap().remove(T::name());
	let data = result?;
	write(&data)?;
	Ok(data)
}
struct Entry {
	loaded: usize,
	target: usize,
	fmt: fn(usize, usize) -> String,
}
// ponytail: global map, single-user dashboard — a lock per source only matters under real concurrency
static REGISTRY: LazyLock<Mutex<HashMap<&'static str, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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
