//! Standardizes polled data sources: persist every poll to `$XDG_DATA_HOME`, and
//! don't hit upstream again until the persisted copy is older than [`SourceData::decay_horizon`].
use std::{
	collections::HashMap,
	future::Future,
	path::PathBuf,
	sync::{Arc, LazyLock, Mutex},
};

use color_eyre::eyre::Result;
use futures::lock::Mutex as AsyncMutex;
use jiff::{SignedDuration, Timestamp};
use serde::{Serialize, de::DeserializeOwned};
use v_utils::trades::Timeframe;

pub trait SourceData: Sized + Serialize + DeserializeOwned + Send {
	/// Refresh interval: once the persisted copy is older than this, the next `load` repolls.
	/// Staleness (the client-facing warning) is a longer horizon — see [`load`].
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
/// A loaded source, plus a marker when the served copy is old enough to surface as stale.
pub struct Loaded<T> {
	pub data: T,
	pub stale: Option<Stale>,
}
pub struct Stale {
	pub fetched_at: Timestamp,
	pub error: String,
}

/// Serve the persisted copy while inside its refresh interval (or always, under mock). Past the
/// interval, refresh under a per-source lock so concurrent callers coalesce into one upstream poll
/// rather than each fanning out. If the refresh fails, still serve the persisted copy — flagged
/// [`Stale`] only once it's older than `interval × stale_multiplier`, so a blip inside the grace
/// window stays quiet.
pub async fn load<T: SourceData>() -> Result<Loaded<T>> {
	let interval = T::decay_horizon().duration();

	// Fast path: a fresh (or mock) copy needs neither the lock nor a poll.
	if let Some(c) = read::<T>() {
		let age = Timestamp::now().duration_since(c.fetched_at);
		if mock_enabled() || !past(age, interval) {
			return Ok(Loaded { data: c.data, stale: None });
		}
	}

	// Refresh due — hold the source lock across the poll so a second caller queues behind us and,
	// on waking, finds the copy we just wrote instead of launching its own fan-out.
	let lock = source_lock::<T>();
	let _guard = lock.lock().await;

	let cached = read::<T>();
	let now_fresh = cached.as_ref().is_some_and(|c| !past(Timestamp::now().duration_since(c.fetched_at), interval));
	if now_fresh {
		return Ok(Loaded {
			data: cached.expect("now_fresh implies Some").data,
			stale: None,
		});
	}

	match fetch_tracked::<T>().await {
		Ok(data) => {
			write(&data)?;
			Ok(Loaded { data, stale: None })
		}
		Err(e) => match cached {
			Some(c) => {
				let stale_after = interval.mul_f64(stale_multiplier());
				let age = Timestamp::now().duration_since(c.fetched_at);
				let stale = past(age, stale_after).then(|| Stale {
					fetched_at: c.fetched_at,
					error: e.to_string(),
				});
				match &stale {
					Some(_) => tracing::warn!("refetch of {} failed ({e}); serving stale copy from {}", T::name(), c.fetched_at),
					None => tracing::debug!("refetch of {} failed ({e}); serving copy from {} (within grace)", T::name(), c.fetched_at),
				}
				Ok(Loaded { data: c.data, stale })
			}
			None => Err(e),
		},
	}
}
/// At/over `horizon` — or negative, since a future `fetched_at` (clock skew) should count as due.
fn past(age: SignedDuration, horizon: std::time::Duration) -> bool {
	age.is_negative() || age.unsigned_abs() >= horizon
}

/// Per-source async lock, created on first use, so `load` coalesces concurrent refreshes into one.
fn source_lock<T: SourceData>() -> Arc<AsyncMutex<()>> {
	LOCKS.lock().unwrap().entry(T::name()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone()
}
static LOCKS: LazyLock<Mutex<HashMap<&'static str, Arc<AsyncMutex<()>>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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

fn stale_multiplier() -> f64 {
	leptos::prelude::use_context::<crate::config::LiveSettings>()
		.expect("LiveSettings in context")
		.config()
		.expect("config loads")
		.stale_multiplier()
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
