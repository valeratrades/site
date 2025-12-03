extern crate clap;

use std::collections::HashMap;

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
				let value = std::env::var(&env_var).map_err(|_| Error::custom(format!("environment variable '{}' not found", env_var)))?;
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
	/// Usernames that have admin access
	#[serde(default)]
	pub users: Vec<String>,
	/// Credentials to display on admin page (values can use { env = "VAR_NAME" })
	#[serde(default)]
	pub creds: Option<HashMap<String, EnvString>>,
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
