//! Standardizes polled data sources: persist every poll to `$XDG_DATA_HOME`, and
//! don't hit upstream again until the persisted copy is older than [`SourceData::decay_horizon`].
use std::{
	collections::HashMap,
	future::Future,
	path::PathBuf,
	sync::{LazyLock, Mutex},
};

use color_eyre::eyre::Result;
use jiff::Timestamp;
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
/// A loaded source, plus a marker when upstream was unreachable and we served the last-good copy.
pub struct Loaded<T> {
	pub data: T,
	pub stale: Option<Stale>,
}
pub struct Stale {
	pub fetched_at: Timestamp,
	pub error: String,
}

/// Serve the persisted copy while fresh (or always, under mock); otherwise repoll and persist.
/// If the repoll fails but a persisted copy exists, serve that copy flagged [`Stale`] rather than
/// erroring — a rate-limit ban shouldn't blank a dashboard that has good data on disk.
pub async fn load<T: SourceData>() -> Result<Loaded<T>> {
	let cached = read::<T>();
	let serve_cached = cached.as_ref().is_some_and(|c| {
		let age = Timestamp::now().duration_since(c.fetched_at);
		mock_enabled() || !(age.is_negative() || age.unsigned_abs() >= T::decay_horizon().duration())
	});
	if serve_cached {
		let c = cached.expect("serve_cached implies Some");
		tracing::debug!("serving persisted {}", T::name());
		return Ok(Loaded { data: c.data, stale: None });
	}
	match fetch_tracked::<T>().await {
		Ok(data) => {
			write(&data)?;
			Ok(Loaded { data, stale: None })
		}
		Err(e) => match cached {
			Some(c) => {
				tracing::warn!("refetch of {} failed ({e}); serving last-good copy from {}", T::name(), c.fetched_at);
				Ok(Loaded {
					data: c.data,
					stale: Some(Stale {
						fetched_at: c.fetched_at,
						error: e.to_string(),
					}),
				})
			}
			None => Err(e),
		},
	}
}

async fn fetch_tracked<T: SourceData>() -> Result<T> {
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
	result
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
	fetched_at: Timestamp,
	data: T,
}
#[derive(Serialize)]
struct CachedRef<'a, T> {
	fetched_at: Timestamp,
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
	let record = CachedRef { fetched_at: Timestamp::now(), data };
	let p = path::<T>();
	std::fs::write(&p, serde_json::to_string_pretty(&record)?)?;
	tracing::info!("persisted {}", T::name());
	Ok(())
}
