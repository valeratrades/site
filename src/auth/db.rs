use clickhouse::Client;
use color_eyre::eyre::{Context, Result};
use tracing::info;

use super::User;
use crate::config::ClickHouseConfig;

const MIGRATIONS: &[&str] = &[
	// Migration 0: Create users table
	r#"
CREATE TABLE IF NOT EXISTS site.users (
    id String,
    email String,
    username String,
    password_hash String,
    created_at DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY id
PRIMARY KEY id
"#,
	// Migration 1: Create sessions table
	r#"
CREATE TABLE IF NOT EXISTS site.sessions (
    session_id String,
    user_id String,
    created_at DateTime DEFAULT now(),
    expires_at DateTime
) ENGINE = MergeTree()
ORDER BY session_id
PRIMARY KEY session_id
"#,
	// Migration 2: Add email_verified column to users
	r#"
ALTER TABLE site.users ADD COLUMN IF NOT EXISTS email_verified UInt8 DEFAULT 0
"#,
	// Migration 3: Create email verification tokens table
	r#"
CREATE TABLE IF NOT EXISTS site.email_tokens (
    token String,
    user_id String,
    created_at DateTime DEFAULT now(),
    expires_at DateTime
) ENGINE = MergeTree()
ORDER BY token
PRIMARY KEY token
"#,
	// Migration 4: Create OAuth state table for CSRF protection
	r#"
CREATE TABLE IF NOT EXISTS site.oauth_states (
    state String,
    created_at DateTime DEFAULT now(),
    expires_at DateTime
) ENGINE = MergeTree()
ORDER BY state
PRIMARY KEY state
"#,
	// Migration 5: Add google_id column for OAuth users
	r#"
ALTER TABLE site.users ADD COLUMN IF NOT EXISTS google_id String DEFAULT ''
"#,
	// Migration 6: Create admin files table
	r#"
CREATE TABLE IF NOT EXISTS site.admin_files (
    id String,
    filename String,
    content_type String,
    data String,
    uploaded_by String,
    uploaded_at DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY (uploaded_at, id)
"#,
];

#[derive(Clone)]
pub struct Database {
	client: Client,
	url: String,
}

impl std::fmt::Debug for Database {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Database").field("url", &self.url).finish()
	}
}

impl Database {
	pub fn new(config: &ClickHouseConfig) -> Self {
		let client = Client::default()
			.with_url(&config.url)
			.with_database(&config.database)
			.with_user(&config.user)
			.with_password(&config.password);

		Self { client, url: config.url.clone() }
	}

	pub async fn migrate(&self) -> Result<()> {
		info!("Running database migrations...");

		self.ensure_database_exists().await?;
		self.ensure_migrations_table_exists().await?;

		let current_version = self.get_migration_version().await?;
		info!("Current migration version: {}", current_version);
		info!("Total migrations available: {}", MIGRATIONS.len());

		let mut applied = 0;
		for (idx, migration) in MIGRATIONS.iter().enumerate() {
			let version = idx as i32;
			if version > current_version {
				info!("Applying migration {}", version);
				self.client.query(migration).execute().await?;
				self.record_migration(version as u32).await?;
				applied += 1;
			}
		}

		if applied > 0 {
			info!("Applied {} migration(s)", applied);
		} else {
			info!("No new migrations to apply");
		}
		info!("Migrations complete");
		Ok(())
	}

	async fn ensure_database_exists(&self) -> Result<()> {
		let client = self.client.clone().with_database("");
		let query = "CREATE DATABASE IF NOT EXISTS site";

		client.query(query).execute().await.map_err(|e| {
			color_eyre::eyre::eyre!(
				"Failed to connect to ClickHouse server.\n\
				\n\
				Possible issues:\n\
				  1. ClickHouse server is not running\n\
				  2. Wrong URL configured (currently: {})\n\
				  3. Network/firewall blocking connection\n\
				\n\
				To fix:\n\
				  - Start ClickHouse: sudo systemctl start clickhouse-server\n\
				  - Check status: sudo systemctl status clickhouse-server\n\
				\n\
				Original error: {:#}",
				self.url,
				e
			)
		})?;
		Ok(())
	}

	async fn ensure_migrations_table_exists(&self) -> Result<()> {
		let query = r#"
CREATE TABLE IF NOT EXISTS site.migrations (
    version UInt32,
    applied_at DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY version
PRIMARY KEY version
"#;
		self.client.query(query).execute().await?;
		Ok(())
	}

	async fn get_migration_version(&self) -> Result<i32> {
		let count_query = "SELECT count() FROM site.migrations";
		let count: u64 = match self.client.query(count_query).fetch_one::<u64>().await {
			Ok(c) => c,
			Err(_) => 0,
		};

		if count == 0 {
			return Ok(-1);
		}

		let version_query = "SELECT max(version) FROM site.migrations";
		let version: u32 = self.client.query(version_query).fetch_one::<u32>().await?;

		Ok(version as i32)
	}

	async fn record_migration(&self, version: u32) -> Result<()> {
		let query = format!("INSERT INTO site.migrations (version) VALUES ({})", version);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn create_user(&self, id: &str, email: &str, username: &str, password: &str) -> Result<()> {
		let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).context("Failed to hash password")?;

		let query = format!(
			"INSERT INTO site.users (id, email, username, password_hash) VALUES ('{}', '{}', '{}', '{}')",
			id.replace('\'', "''"),
			email.replace('\'', "''"),
			username.replace('\'', "''"),
			password_hash.replace('\'', "''")
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn get_user_by_email(&self, email: &str) -> Result<Option<(User, String)>> {
		let query = format!("SELECT id, email, username, password_hash FROM site.users WHERE email = '{}' LIMIT 1", email.replace('\'', "''"));

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct UserRow {
			id: String,
			email: String,
			username: String,
			password_hash: String,
		}

		match self.client.query(&query).fetch_one::<UserRow>().await {
			Ok(row) => Ok(Some((
				User {
					id: row.id,
					email: row.email,
					username: row.username,
				},
				row.password_hash,
			))),
			Err(_) => Ok(None),
		}
	}

	pub async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
		let query = format!(
			"SELECT id, email, username, password_hash FROM site.users WHERE username = '{}' LIMIT 1",
			username.replace('\'', "''")
		);

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct UserRow {
			id: String,
			email: String,
			username: String,
			password_hash: String,
		}

		match self.client.query(&query).fetch_one::<UserRow>().await {
			Ok(row) => Ok(Some((
				User {
					id: row.id,
					email: row.email,
					username: row.username,
				},
				row.password_hash,
			))),
			Err(_) => Ok(None),
		}
	}

	pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
		let query = format!("SELECT id, email, username FROM site.users WHERE id = '{}' LIMIT 1", id.replace('\'', "''"));

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct UserRow {
			id: String,
			email: String,
			username: String,
		}

		match self.client.query(&query).fetch_one::<UserRow>().await {
			Ok(row) => Ok(Some(User {
				id: row.id,
				email: row.email,
				username: row.username,
			})),
			Err(_) => Ok(None),
		}
	}

	pub async fn email_exists(&self, email: &str) -> Result<bool> {
		let query = format!("SELECT count() FROM site.users WHERE email = '{}'", email.replace('\'', "''"));
		let count: u64 = self.client.query(&query).fetch_one::<u64>().await?;
		Ok(count > 0)
	}

	pub async fn create_session(&self, session_id: &str, user_id: &str, expires_hours: u32) -> Result<()> {
		let query = format!(
			"INSERT INTO site.sessions (session_id, user_id, expires_at) VALUES ('{}', '{}', now() + INTERVAL {} HOUR)",
			session_id.replace('\'', "''"),
			user_id.replace('\'', "''"),
			expires_hours
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn get_session_user(&self, session_id: &str) -> Result<Option<User>> {
		let query = format!(
			"SELECT u.id, u.email, u.username FROM site.sessions s \
			 JOIN site.users u ON s.user_id = u.id \
			 WHERE s.session_id = '{}' AND s.expires_at > now() \
			 LIMIT 1",
			session_id.replace('\'', "''")
		);

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct UserRow {
			id: String,
			email: String,
			username: String,
		}

		match self.client.query(&query).fetch_one::<UserRow>().await {
			Ok(row) => Ok(Some(User {
				id: row.id,
				email: row.email,
				username: row.username,
			})),
			Err(_) => Ok(None),
		}
	}

	pub async fn delete_session(&self, session_id: &str) -> Result<()> {
		let query = format!("ALTER TABLE site.sessions DELETE WHERE session_id = '{}'", session_id.replace('\'', "''"));
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn create_email_token(&self, token: &str, user_id: &str, expires_hours: u32) -> Result<()> {
		let query = format!(
			"INSERT INTO site.email_tokens (token, user_id, expires_at) VALUES ('{}', '{}', now() + INTERVAL {} HOUR)",
			token.replace('\'', "''"),
			user_id.replace('\'', "''"),
			expires_hours
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn verify_email_token(&self, token: &str) -> Result<Option<String>> {
		let query = format!(
			"SELECT user_id FROM site.email_tokens WHERE token = '{}' AND expires_at > now() LIMIT 1",
			token.replace('\'', "''")
		);

		match self.client.query(&query).fetch_one::<String>().await {
			Ok(user_id) => Ok(Some(user_id)),
			Err(_) => Ok(None),
		}
	}

	pub async fn mark_email_verified(&self, user_id: &str) -> Result<()> {
		let query = format!("ALTER TABLE site.users UPDATE email_verified = 1 WHERE id = '{}'", user_id.replace('\'', "''"));
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn delete_email_token(&self, token: &str) -> Result<()> {
		let query = format!("ALTER TABLE site.email_tokens DELETE WHERE token = '{}'", token.replace('\'', "''"));
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn is_email_verified(&self, user_id: &str) -> Result<bool> {
		let query = format!("SELECT email_verified FROM site.users WHERE id = '{}' LIMIT 1", user_id.replace('\'', "''"));
		let verified: u8 = self.client.query(&query).fetch_one::<u8>().await.unwrap_or(0);
		Ok(verified == 1)
	}

	// OAuth state management
	pub async fn create_oauth_state(&self, state: &str, expires_minutes: u32) -> Result<()> {
		let query = format!(
			"INSERT INTO site.oauth_states (state, expires_at) VALUES ('{}', now() + INTERVAL {} MINUTE)",
			state.replace('\'', "''"),
			expires_minutes
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn verify_oauth_state(&self, state: &str) -> Result<bool> {
		let query = format!("SELECT count() FROM site.oauth_states WHERE state = '{}' AND expires_at > now()", state.replace('\'', "''"));
		let count: u64 = self.client.query(&query).fetch_one::<u64>().await.unwrap_or(0);
		Ok(count > 0)
	}

	pub async fn delete_oauth_state(&self, state: &str) -> Result<()> {
		let query = format!("ALTER TABLE site.oauth_states DELETE WHERE state = '{}'", state.replace('\'', "''"));
		self.client.query(&query).execute().await?;
		Ok(())
	}

	// Google OAuth user management
	pub async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>> {
		let query = format!("SELECT id, email, username FROM site.users WHERE google_id = '{}' LIMIT 1", google_id.replace('\'', "''"));

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct UserRow {
			id: String,
			email: String,
			username: String,
		}

		match self.client.query(&query).fetch_one::<UserRow>().await {
			Ok(row) => Ok(Some(User {
				id: row.id,
				email: row.email,
				username: row.username,
			})),
			Err(_) => Ok(None),
		}
	}

	pub async fn create_google_user(&self, id: &str, email: &str, username: &str, google_id: &str) -> Result<()> {
		let query = format!(
			"INSERT INTO site.users (id, email, username, password_hash, email_verified, google_id) VALUES ('{}', '{}', '{}', '', 1, '{}')",
			id.replace('\'', "''"),
			email.replace('\'', "''"),
			username.replace('\'', "''"),
			google_id.replace('\'', "''")
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn link_google_to_user(&self, user_id: &str, google_id: &str) -> Result<()> {
		let query = format!(
			"ALTER TABLE site.users UPDATE google_id = '{}', email_verified = 1 WHERE id = '{}'",
			google_id.replace('\'', "''"),
			user_id.replace('\'', "''")
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	// Admin files management
	pub async fn create_admin_file(&self, id: &str, filename: &str, content_type: &str, data: &str, uploaded_by: &str) -> Result<()> {
		let query = format!(
			"INSERT INTO site.admin_files (id, filename, content_type, data, uploaded_by) VALUES ('{}', '{}', '{}', '{}', '{}')",
			id.replace('\'', "''"),
			filename.replace('\'', "''"),
			content_type.replace('\'', "''"),
			data.replace('\'', "''"),
			uploaded_by.replace('\'', "''")
		);
		self.client.query(&query).execute().await?;
		Ok(())
	}

	pub async fn list_admin_files(&self) -> Result<Vec<AdminFile>> {
		let query = "SELECT id, filename, content_type, uploaded_by, toString(uploaded_at) as uploaded_at FROM site.admin_files ORDER BY uploaded_at DESC";

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct FileRow {
			id: String,
			filename: String,
			content_type: String,
			uploaded_by: String,
			uploaded_at: String,
		}

		let rows: Vec<FileRow> = self.client.query(query).fetch_all().await.unwrap_or_default();
		Ok(rows
			.into_iter()
			.map(|r| AdminFile {
				id: r.id,
				filename: r.filename,
				content_type: r.content_type,
				uploaded_by: r.uploaded_by,
				uploaded_at: r.uploaded_at,
			})
			.collect())
	}

	pub async fn get_admin_file(&self, id: &str) -> Result<Option<AdminFileWithData>> {
		let query = format!(
			"SELECT id, filename, content_type, data, uploaded_by, toString(uploaded_at) as uploaded_at FROM site.admin_files WHERE id = '{}' LIMIT 1",
			id.replace('\'', "''")
		);

		#[derive(serde::Deserialize, clickhouse::Row)]
		struct FileRow {
			id: String,
			filename: String,
			content_type: String,
			data: String,
			uploaded_by: String,
			uploaded_at: String,
		}

		match self.client.query(&query).fetch_one::<FileRow>().await {
			Ok(r) => Ok(Some(AdminFileWithData {
				id: r.id,
				filename: r.filename,
				content_type: r.content_type,
				data: r.data,
				uploaded_by: r.uploaded_by,
				uploaded_at: r.uploaded_at,
			})),
			Err(_) => Ok(None),
		}
	}

	pub async fn delete_admin_file(&self, id: &str) -> Result<()> {
		let query = format!("ALTER TABLE site.admin_files DELETE WHERE id = '{}'", id.replace('\'', "''"));
		self.client.query(&query).execute().await?;
		Ok(())
	}
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminFile {
	pub id: String,
	pub filename: String,
	pub content_type: String,
	pub uploaded_by: String,
	pub uploaded_at: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminFileWithData {
	pub id: String,
	pub filename: String,
	pub content_type: String,
	pub data: String,
	pub uploaded_by: String,
	pub uploaded_at: String,
}
