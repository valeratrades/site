extern crate clap;

use std::collections::HashMap;
#[cfg(feature = "ssr")]
use std::{
	path::PathBuf,
	sync::{Arc, RwLock},
	time::{Duration, SystemTime},
};

/// A string that can be either a direct value or resolved from an environment variable.
/// Deserializes from either `"value"` or `{ env = "VAR_NAME" }`.
#[derive(Clone, Debug)]
pub struct EnvString(pub String);

impl<'de> serde::Deserialize<'de> for EnvString {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::de::Deserializer<'de>, {
		use serde::de::{Error, MapAccess, Visitor};

		struct EnvStringVisitor;

		impl<'de> Visitor<'de> for EnvStringVisitor {
			type Value = EnvString;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a string or a map with key 'env'")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: Error, {
				Ok(EnvString(value.to_owned()))
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>, {
				let mut env_var: Option<String> = None;
				while let Some(key) = map.next_key::<String>()? {
					if key == "env" {
						env_var = Some(map.next_value()?);
					} else {
						let _: serde::de::IgnoredAny = map.next_value()?;
					}
				}
				let env_var = env_var.ok_or_else(|| Error::custom("expected 'env' key"))?;
				let value = std::env::var(&env_var).map_err(|_| Error::custom(format!("environment variable '{env_var}' not found")))?;
				Ok(EnvString(value))
			}
		}

		deserializer.deserialize_any(EnvStringVisitor)
	}
}

#[derive(Clone, Debug, v_utils::macros::MyConfigPrimitives)]
#[cfg_attr(feature = "ssr", derive(v_utils::macros::Settings))]
pub struct Settings {
	pub mock: Option<bool>,
	#[serde(default)]
	pub clickhouse: ClickHouseConfig,
	#[serde(default)]
	pub smtp: SmtpConfig,
	#[serde(default)]
	pub google_oauth: GoogleOAuthConfig,
	/// Base URL for the site (used in email links)
	#[serde(default = "__default_site_url")]
	#[primitives(skip)]
	pub site_url: String,
	/// Admin configuration
	#[serde(default)]
	#[primitives(skip)]
	pub admin: AdminConf,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct AdminConf {
	/// Usernames that have admin access, mapped to their permission level (0.0 to 1.0)
	#[serde(default)]
	pub users: HashMap<String, v_utils::percent::PercentU>,
	/// Credentials to display on admin page, nested by permission level (0-100).
	/// Only admins with >= permission level see each credential.
	/// Format: { "100" = { "secret" = "value" }, "50" = { "less_secret" = "value" } }
	#[serde(default)]
	pub creds: Option<HashMap<String, HashMap<String, EnvString>>>,
}

fn __default_site_url() -> String {
	"http://localhost:61156".to_string()
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			mock: Some(false),
			clickhouse: ClickHouseConfig::default(),
			smtp: SmtpConfig::default(),
			google_oauth: GoogleOAuthConfig::default(),
			site_url: __default_site_url(),
			admin: AdminConf::default(),
		}
	}
}

impl Settings {
	pub fn mock(&self) -> bool {
		self.mock.unwrap_or(false)
	}
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ClickHouseConfig {
	#[serde(default = "__default_clickhouse_url")]
	pub url: String,
	#[serde(default = "__default_clickhouse_database")]
	pub database: String,
	#[serde(default = "__default_clickhouse_user")]
	pub user: String,
	#[serde(default)]
	pub password: String,
}

impl Default for ClickHouseConfig {
	fn default() -> Self {
		Self {
			url: __default_clickhouse_url(),
			database: __default_clickhouse_database(),
			user: __default_clickhouse_user(),
			password: String::new(),
		}
	}
}

fn __default_clickhouse_url() -> String {
	"http://localhost:8123".to_string()
}

fn __default_clickhouse_database() -> String {
	"site".to_string()
}

fn __default_clickhouse_user() -> String {
	"default".to_string()
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SmtpConfig {
	pub host: String,
	pub port: u16,
	pub username: String,
	pub password: String,
	pub from_email: String,
	pub from_name: String,
}

impl Default for SmtpConfig {
	fn default() -> Self {
		Self {
			host: "smtp.gmail.com".to_string(),
			port: 587,
			username: String::new(),
			password: String::new(),
			from_email: String::new(),
			from_name: "My Site".to_string(),
		}
	}
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct GoogleOAuthConfig {
	/// Google OAuth2 Client ID (from Google Cloud Console)
	pub client_id: String,
	/// Google OAuth2 Client Secret (from Google Cloud Console)
	pub client_secret: String,
}

impl GoogleOAuthConfig {
	pub fn is_configured(&self) -> bool {
		!self.client_id.is_empty() && !self.client_secret.is_empty()
	}
}

// === Hot-reloading Settings wrapper ===

#[cfg(feature = "ssr")]
struct TimeCapsule {
	value: Settings,
	loaded_at: SystemTime,
	update_freq: Duration,
}

/// Thread-safe Settings wrapper with automatic config file hot-reload.
/// When `config()` is called, checks if the config file has been modified
/// since last load and reloads if necessary (with configurable throttling).
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct LiveSettings {
	/// Path to the config file (None if no file was found/used)
	config_path: Option<PathBuf>,
	/// Shared state with cached settings and timing info
	inner: Arc<RwLock<TimeCapsule>>,
	/// Original CLI flags for rebuilding config
	flags: SettingsFlags,
}

#[cfg(feature = "ssr")]
impl std::fmt::Debug for LiveSettings {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LiveSettings").field("config_path", &self.config_path).finish()
	}
}

#[cfg(feature = "ssr")]
impl LiveSettings {
	/// Create a new LiveSettings from CLI flags.
	/// `update_freq` controls how often the file modification time is checked.
	pub fn new(flags: SettingsFlags, update_freq: Duration) -> eyre::Result<Self> {
		let config_path = Self::resolve_config_path(&flags);
		let settings = Settings::try_build(flags.clone())?;

		Ok(Self {
			config_path,
			inner: Arc::new(RwLock::new(TimeCapsule {
				value: settings,
				loaded_at: SystemTime::now(),
				update_freq,
			})),
			flags,
		})
	}

	/// Resolve the config file path using the same logic as v_utils Settings macro.
	fn resolve_config_path(flags: &SettingsFlags) -> Option<PathBuf> {
		// If explicit path provided via CLI, use that
		if let Some(ref path) = flags.config {
			return Some(path.0.clone());
		}

		let app_name = env!("CARGO_PKG_NAME");
		let xdg_conf_dir = xdg::BaseDirectories::with_prefix(app_name)
			.get_config_home()
			.and_then(|p: PathBuf| p.parent().map(|p: &std::path::Path| p.to_path_buf()))
			.unwrap_or_else(|| PathBuf::from(std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()))));

		// Check for .nix file first
		let nix_path = xdg_conf_dir.join(format!("{app_name}.nix"));
		if nix_path.exists() {
			return Some(nix_path);
		}

		// Check standard locations
		let location_bases = [xdg_conf_dir.join(app_name), xdg_conf_dir.join(app_name).join("config")];
		let supported_exts = ["toml", "json", "yaml", "json5", "ron", "ini"];

		for base in location_bases.iter() {
			for ext in supported_exts.iter() {
				let path = PathBuf::from(base).with_extension(ext);
				if path.exists() {
					return Some(path);
				}
			}
		}

		None
	}

	/// Get the current settings, reloading from file if it has changed.
	/// This is the hot-reload entry point.
	pub fn config(&self) -> Settings {
		let now = SystemTime::now();

		// First, check if we need to reload (fast path with read lock)
		let should_reload = {
			let capsule = self.inner.read().unwrap();
			let age = now.duration_since(capsule.loaded_at).unwrap_or_default();

			// Only check file if enough time has passed since last check
			if age < capsule.update_freq {
				return capsule.value.clone();
			}

			// Check if file was modified since we loaded
			self.config_path
				.as_ref()
				.and_then(|path| std::fs::metadata(path).ok())
				.and_then(|meta| meta.modified().ok())
				.map(|file_mtime| {
					let since_file_change = now.duration_since(file_mtime).unwrap_or_default();
					since_file_change < age
				})
				.unwrap_or(false)
		};

		if should_reload {
			// Attempt to reload config
			if let Ok(new_settings) = Settings::try_build(self.flags.clone()) {
				let mut capsule = self.inner.write().unwrap();
				capsule.value = new_settings;
				capsule.loaded_at = now;
				tracing::info!("Config reloaded from {:?}", self.config_path);
			} else {
				// Reload failed, just update the timestamp to avoid repeated attempts
				let mut capsule = self.inner.write().unwrap();
				capsule.loaded_at = now;
				tracing::warn!("Config reload failed, keeping previous settings");
			}
		} else {
			// Just update the check timestamp
			let mut capsule = self.inner.write().unwrap();
			capsule.loaded_at = now;
		}

		self.inner.read().unwrap().value.clone()
	}

	/// Get a direct reference to the initial settings (no reload check).
	/// Use this for one-time setup that doesn't need hot-reload.
	pub fn initial(&self) -> Settings {
		self.inner.read().unwrap().value.clone()
	}
}

#[cfg(feature = "ssr")]
use color_eyre::eyre;
#[cfg(feature = "ssr")]
use xdg;
