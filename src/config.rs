extern crate clap;

use std::collections::HashMap;

#[derive(Clone, Debug, v_utils::macros::MyConfigPrimitives, serde::Serialize)]
#[cfg_attr(feature = "ssr", derive(v_utils::macros::Settings, v_utils::macros::LiveSettings))]
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

fn __default_site_url() -> String {
	"http://localhost:61156".to_string()
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct AdminConf {
	/// Usernames that have admin access, mapped to their permission level (0.0 to 1.0)
	#[serde(default)]
	pub users: HashMap<String, v_utils::percent::PercentU>,
	/// Credentials to display on admin page, nested by permission level (0-100).
	/// Only admins with >= permission level see each credential.
	/// Format: { "100" = { "secret" = "value" }, "50" = { "less_secret" = "value" } }
	#[serde(default)]
	pub creds: Option<HashMap<String, HashMap<String, String>>>,
}

// No secrets here, plain serde is fine
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
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

// MyConfigPrimitives enables `{ env = "VAR" }` for plain String fields
#[derive(Clone, Debug, v_utils::macros::MyConfigPrimitives, serde::Serialize, v_utils::macros::SettingsNested)]
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
			from_name: String::new(),
		}
	}
}

#[derive(Clone, Debug, Default, v_utils::macros::MyConfigPrimitives, serde::Serialize, v_utils::macros::SettingsNested)]
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
