use chrono::Utc;
use color_eyre::eyre::{Result, WrapErr};
use sqlx::{Row, SqlitePool};
use tracing::info;

use super::User;

fn none_if_empty(s: String) -> Option<String> {
	if s.is_empty() { None } else { Some(s) }
}

fn expires_at_hours(hours: u32) -> String {
	(Utc::now() + chrono::Duration::hours(hours as i64)).format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn expires_at_minutes(minutes: u32) -> String {
	(Utc::now() + chrono::Duration::minutes(minutes as i64)).format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[derive(Clone)]
pub struct Database {
	pool: SqlitePool,
}

impl std::fmt::Debug for Database {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Database").finish()
	}
}

impl Database {
	pub async fn try_new() -> Result<Self> {
		let app_name = env!("CARGO_PKG_NAME");
		let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
		let db_path = xdg_dirs.place_state_file("db.sqlite3")?;
		info!("Opening SQLite database at {}", db_path.display());

		let url = format!("sqlite://{}?mode=rwc", db_path.display());
		let pool = SqlitePool::connect(&url).await.wrap_err("failed to open SQLite database")?;

		// Enable WAL mode for Litestream compatibility
		sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await.wrap_err("failed to set WAL mode")?;

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                username TEXT NOT NULL,
                password_hash TEXT NOT NULL DEFAULT '',
                email_verified INTEGER NOT NULL DEFAULT 0,
                google_id TEXT NOT NULL DEFAULT '',
                display_name TEXT NOT NULL DEFAULT '',
                avatar_url TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
		)
		.execute(&pool)
		.await
		.wrap_err("failed to create users table")?;

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                expires_at TEXT NOT NULL
            )",
		)
		.execute(&pool)
		.await
		.wrap_err("failed to create sessions table")?;

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS email_tokens (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                expires_at TEXT NOT NULL
            )",
		)
		.execute(&pool)
		.await
		.wrap_err("failed to create email_tokens table")?;

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS oauth_states (
                state TEXT PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                expires_at TEXT NOT NULL
            )",
		)
		.execute(&pool)
		.await
		.wrap_err("failed to create oauth_states table")?;

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS admin_files (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                content_type TEXT NOT NULL,
                data TEXT NOT NULL,
                uploaded_by TEXT NOT NULL,
                uploaded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
		)
		.execute(&pool)
		.await
		.wrap_err("failed to create admin_files table")?;

		Ok(Self { pool })
	}

	pub async fn create_user(&self, id: &str, email: &str, username: &str, password: &str) -> Result<()> {
		let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).wrap_err("failed to hash password")?;
		sqlx::query("INSERT INTO users (id, email, username, password_hash) VALUES (?, ?, ?, ?)")
			.bind(id)
			.bind(email)
			.bind(username)
			.bind(&password_hash)
			.execute(&self.pool)
			.await
			.wrap_err("failed to create user")?;
		Ok(())
	}

	pub async fn get_user_by_email(&self, email: &str) -> Result<Option<(User, String)>> {
		let row = sqlx::query("SELECT id, email, username, password_hash, display_name, avatar_url FROM users WHERE email = ? LIMIT 1")
			.bind(email)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to query user by email")?;

		Ok(row.map(|r| {
			(
				User {
					id: r.get("id"),
					email: r.get("email"),
					username: r.get("username"),
					display_name: none_if_empty(r.get("display_name")),
					avatar_url: none_if_empty(r.get("avatar_url")),
				},
				r.get("password_hash"),
			)
		}))
	}

	pub async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
		let row = sqlx::query("SELECT id, email, username, password_hash, display_name, avatar_url FROM users WHERE username = ? LIMIT 1")
			.bind(username)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to query user by username")?;

		Ok(row.map(|r| {
			(
				User {
					id: r.get("id"),
					email: r.get("email"),
					username: r.get("username"),
					display_name: none_if_empty(r.get("display_name")),
					avatar_url: none_if_empty(r.get("avatar_url")),
				},
				r.get("password_hash"),
			)
		}))
	}

	pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
		let row = sqlx::query("SELECT id, email, username, display_name, avatar_url FROM users WHERE id = ? LIMIT 1")
			.bind(id)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to query user by id")?;

		Ok(row.map(|r| User {
			id: r.get("id"),
			email: r.get("email"),
			username: r.get("username"),
			display_name: none_if_empty(r.get("display_name")),
			avatar_url: none_if_empty(r.get("avatar_url")),
		}))
	}

	pub async fn email_exists(&self, email: &str) -> Result<bool> {
		let row = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE email = ?")
			.bind(email)
			.fetch_one(&self.pool)
			.await
			.wrap_err("failed to check email existence")?;
		let count: i64 = row.get("cnt");
		Ok(count > 0)
	}

	pub async fn create_session(&self, session_id: &str, user_id: &str, expires_hours: u32) -> Result<()> {
		sqlx::query("INSERT INTO sessions (session_id, user_id, expires_at) VALUES (?, ?, ?)")
			.bind(session_id)
			.bind(user_id)
			.bind(expires_at_hours(expires_hours))
			.execute(&self.pool)
			.await
			.wrap_err("failed to create session")?;
		Ok(())
	}

	pub async fn get_session_user(&self, session_id: &str) -> Result<Option<User>> {
		let row = sqlx::query(
			"SELECT u.id, u.email, u.username, u.display_name, u.avatar_url \
             FROM sessions s JOIN users u ON s.user_id = u.id \
             WHERE s.session_id = ? AND s.expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
             LIMIT 1",
		)
		.bind(session_id)
		.fetch_optional(&self.pool)
		.await
		.wrap_err("failed to get session user")?;

		Ok(row.map(|r| User {
			id: r.get("id"),
			email: r.get("email"),
			username: r.get("username"),
			display_name: none_if_empty(r.get("display_name")),
			avatar_url: none_if_empty(r.get("avatar_url")),
		}))
	}

	pub async fn delete_session(&self, session_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE session_id = ?")
			.bind(session_id)
			.execute(&self.pool)
			.await
			.wrap_err("failed to delete session")?;
		Ok(())
	}

	pub async fn create_email_token(&self, token: &str, user_id: &str, expires_hours: u32) -> Result<()> {
		sqlx::query("INSERT INTO email_tokens (token, user_id, expires_at) VALUES (?, ?, ?)")
			.bind(token)
			.bind(user_id)
			.bind(expires_at_hours(expires_hours))
			.execute(&self.pool)
			.await
			.wrap_err("failed to create email token")?;
		Ok(())
	}

	pub async fn verify_email_token(&self, token: &str) -> Result<Option<String>> {
		let row = sqlx::query("SELECT user_id FROM email_tokens WHERE token = ? AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now') LIMIT 1")
			.bind(token)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to verify email token")?;

		Ok(row.map(|r| r.get("user_id")))
	}

	pub async fn mark_email_verified(&self, user_id: &str) -> Result<()> {
		sqlx::query("UPDATE users SET email_verified = 1 WHERE id = ?")
			.bind(user_id)
			.execute(&self.pool)
			.await
			.wrap_err("failed to mark email verified")?;
		Ok(())
	}

	pub async fn delete_email_token(&self, token: &str) -> Result<()> {
		sqlx::query("DELETE FROM email_tokens WHERE token = ?")
			.bind(token)
			.execute(&self.pool)
			.await
			.wrap_err("failed to delete email token")?;
		Ok(())
	}

	pub async fn is_email_verified(&self, user_id: &str) -> Result<bool> {
		let row = sqlx::query("SELECT email_verified FROM users WHERE id = ? LIMIT 1")
			.bind(user_id)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to check email verification")?;
		Ok(row.map(|r| r.get::<i64, _>("email_verified") != 0).unwrap_or(false))
	}

	pub async fn create_oauth_state(&self, state: &str, expires_minutes: u32) -> Result<()> {
		sqlx::query("INSERT INTO oauth_states (state, expires_at) VALUES (?, ?)")
			.bind(state)
			.bind(expires_at_minutes(expires_minutes))
			.execute(&self.pool)
			.await
			.wrap_err("failed to create oauth state")?;
		Ok(())
	}

	pub async fn verify_oauth_state(&self, state: &str) -> Result<bool> {
		let row = sqlx::query("SELECT COUNT(*) as cnt FROM oauth_states WHERE state = ? AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
			.bind(state)
			.fetch_one(&self.pool)
			.await
			.wrap_err("failed to verify oauth state")?;
		let count: i64 = row.get("cnt");
		Ok(count > 0)
	}

	pub async fn delete_oauth_state(&self, state: &str) -> Result<()> {
		sqlx::query("DELETE FROM oauth_states WHERE state = ?")
			.bind(state)
			.execute(&self.pool)
			.await
			.wrap_err("failed to delete oauth state")?;
		Ok(())
	}

	pub async fn get_user_by_google_id(&self, google_id: &str) -> Result<Option<User>> {
		let row = sqlx::query("SELECT id, email, username, display_name, avatar_url FROM users WHERE google_id = ? LIMIT 1")
			.bind(google_id)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to query user by google id")?;

		Ok(row.map(|r| User {
			id: r.get("id"),
			email: r.get("email"),
			username: r.get("username"),
			display_name: none_if_empty(r.get("display_name")),
			avatar_url: none_if_empty(r.get("avatar_url")),
		}))
	}

	pub async fn create_google_user(&self, id: &str, email: &str, username: &str, google_id: &str, display_name: &str, avatar_url: &str) -> Result<()> {
		sqlx::query(
			"INSERT INTO users (id, email, username, password_hash, email_verified, google_id, display_name, avatar_url) \
             VALUES (?, ?, ?, '', 1, ?, ?, ?)",
		)
		.bind(id)
		.bind(email)
		.bind(username)
		.bind(google_id)
		.bind(display_name)
		.bind(avatar_url)
		.execute(&self.pool)
		.await
		.wrap_err("failed to create google user")?;
		Ok(())
	}

	pub async fn link_google_to_user(&self, user_id: &str, google_id: &str, avatar_url: &str, display_name: &str) -> Result<()> {
		sqlx::query("UPDATE users SET google_id = ?, email_verified = 1, avatar_url = ?, display_name = ? WHERE id = ?")
			.bind(google_id)
			.bind(avatar_url)
			.bind(display_name)
			.bind(user_id)
			.execute(&self.pool)
			.await
			.wrap_err("failed to link google to user")?;
		Ok(())
	}

	pub async fn update_google_user_info(&self, user_id: &str, avatar_url: &str, display_name: &str) -> Result<()> {
		sqlx::query("UPDATE users SET avatar_url = ?, display_name = ? WHERE id = ?")
			.bind(avatar_url)
			.bind(display_name)
			.bind(user_id)
			.execute(&self.pool)
			.await
			.wrap_err("failed to update google user info")?;
		Ok(())
	}

	pub async fn update_username(&self, user_id: &str, new_username: &str) -> Result<()> {
		sqlx::query("UPDATE users SET username = ? WHERE id = ?")
			.bind(new_username)
			.bind(user_id)
			.execute(&self.pool)
			.await
			.wrap_err("failed to update username")?;
		Ok(())
	}

	pub async fn username_exists(&self, username: &str) -> Result<bool> {
		let row = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE username = ?")
			.bind(username)
			.fetch_one(&self.pool)
			.await
			.wrap_err("failed to check username existence")?;
		let count: i64 = row.get("cnt");
		Ok(count > 0)
	}

	pub async fn create_admin_file(&self, id: &str, filename: &str, content_type: &str, data: &str, uploaded_by: &str) -> Result<()> {
		sqlx::query("INSERT INTO admin_files (id, filename, content_type, data, uploaded_by) VALUES (?, ?, ?, ?, ?)")
			.bind(id)
			.bind(filename)
			.bind(content_type)
			.bind(data)
			.bind(uploaded_by)
			.execute(&self.pool)
			.await
			.wrap_err("failed to create admin file")?;
		Ok(())
	}

	pub async fn list_admin_files(&self) -> Result<Vec<AdminFile>> {
		let rows = sqlx::query("SELECT id, filename, content_type, uploaded_by, uploaded_at FROM admin_files ORDER BY uploaded_at DESC")
			.fetch_all(&self.pool)
			.await
			.wrap_err("failed to list admin files")?;

		Ok(rows
			.into_iter()
			.map(|r| AdminFile {
				id: r.get("id"),
				filename: r.get("filename"),
				content_type: r.get("content_type"),
				uploaded_by: r.get("uploaded_by"),
				uploaded_at: r.get("uploaded_at"),
			})
			.collect())
	}

	pub async fn get_admin_file(&self, id: &str) -> Result<Option<AdminFileWithData>> {
		let row = sqlx::query("SELECT id, filename, content_type, data, uploaded_by, uploaded_at FROM admin_files WHERE id = ? LIMIT 1")
			.bind(id)
			.fetch_optional(&self.pool)
			.await
			.wrap_err("failed to get admin file")?;

		Ok(row.map(|r| AdminFileWithData {
			id: r.get("id"),
			filename: r.get("filename"),
			content_type: r.get("content_type"),
			data: r.get("data"),
			uploaded_by: r.get("uploaded_by"),
			uploaded_at: r.get("uploaded_at"),
		}))
	}

	pub async fn delete_admin_file(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM admin_files WHERE id = ?")
			.bind(id)
			.execute(&self.pool)
			.await
			.wrap_err("failed to delete admin file")?;
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
