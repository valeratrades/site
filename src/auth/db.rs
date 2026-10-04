use color_eyre::eyre::{Result, WrapErr};
use jiff::{Timestamp, ToSpan};
use sqlx::{Row, SqlitePool};
use tracing::info;

use super::User;

#[derive(Clone)]
pub struct Database {
	pool: SqlitePool,
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

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS group_members (
                grp TEXT NOT NULL,
                email TEXT NOT NULL,
                added_by TEXT NOT NULL,
                added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                PRIMARY KEY (grp, email)
            )",
		)
		.execute(&pool)
		.await
		.wrap_err("failed to create group_members table")?;

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

	/// The lookup every `get_user_by_*` shares. `query` stays a literal at each call site so sqlx's
	/// injection guard still applies — only the fetch and the row mapping are common.
	async fn user_row(&self, query: &'static str, value: &str, what: &'static str) -> Result<Option<sqlx::sqlite::SqliteRow>> {
		sqlx::query(query).bind(value).fetch_optional(&self.pool).await.wrap_err(what)
	}

	pub async fn get_user_by_email(&self, email: &str) -> Result<Option<(User, String)>> {
		let row = self
			.user_row(
				"SELECT id, email, username, password_hash, display_name, avatar_url FROM users WHERE email = ? LIMIT 1",
				email,
				"failed to query user by email",
			)
			.await?;
		Ok(row.map(|r| (user_from_row(&r), r.get("password_hash"))))
	}

	/// Case-insensitive: config lists and sign-ups need not agree on case.
	pub async fn get_verified_user_by_email(&self, email: &str) -> Result<Option<User>> {
		let row = self
			.user_row(
				"SELECT id, email, username, display_name, avatar_url FROM users WHERE lower(email) = lower(?) AND email_verified = 1 LIMIT 1",
				email,
				"failed to query verified user by email",
			)
			.await?;
		Ok(row.as_ref().map(user_from_row))
	}

	pub async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
		let row = self
			.user_row(
				"SELECT id, email, username, password_hash, display_name, avatar_url FROM users WHERE username = ? LIMIT 1",
				username,
				"failed to query user by username",
			)
			.await?;
		Ok(row.map(|r| (user_from_row(&r), r.get("password_hash"))))
	}

	pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
		let row = self
			.user_row(
				"SELECT id, email, username, display_name, avatar_url FROM users WHERE id = ? LIMIT 1",
				id,
				"failed to query user by id",
			)
			.await?;
		Ok(row.map(|r| user_from_row(&r)))
	}

	pub async fn email_exists(&self, email: &str) -> Result<bool> {
		let row = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE lower(email) = lower(?)")
			.bind(email)
			.fetch_one(&self.pool)
			.await
			.wrap_err("failed to check email existence")?;
		let count: i64 = row.get("cnt");
		Ok(count > 0)
	}

	/// An unverified account under `email`, with its sessions and tokens: it could be anyone's.
	pub async fn drop_unverified(&self, email: &str) -> Result<()> {
		let mut tx = self.pool.begin().await.wrap_err("failed to begin")?;
		for q in [
			"DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE lower(email) = lower(?) AND email_verified = 0)",
			"DELETE FROM email_tokens WHERE user_id IN (SELECT id FROM users WHERE lower(email) = lower(?) AND email_verified = 0)",
			"DELETE FROM users WHERE lower(email) = lower(?) AND email_verified = 0",
		] {
			sqlx::query(q).bind(email).execute(&mut *tx).await.wrap_err("failed to drop unverified account")?;
		}
		tx.commit().await.wrap_err("failed to commit")?;
		Ok(())
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
		let row = self
			.user_row(
				"SELECT id, email, username, display_name, avatar_url FROM users WHERE google_id = ? LIMIT 1",
				google_id,
				"failed to query user by google id",
			)
			.await?;
		Ok(row.map(|r| user_from_row(&r)))
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
		// a password set before the email was proven could be anyone's
		sqlx::query(
			"UPDATE users SET google_id = ?, password_hash = CASE email_verified WHEN 1 THEN password_hash ELSE '' END, email_verified = 1, avatar_url = ?, display_name = ? WHERE id = ?",
		)
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

	/// Idempotent; emails are kept lowercase.
	pub async fn add_to_group(&self, group: &str, email: &str, by: &str) -> Result<()> {
		sqlx::query("INSERT INTO group_members (grp, email, added_by) VALUES (?, lower(?), lower(?)) ON CONFLICT DO NOTHING")
			.bind(group)
			.bind(email)
			.bind(by)
			.execute(&self.pool)
			.await
			.wrap_err("failed to add to group")?;
		Ok(())
	}

	/// Whether the email was in the group.
	pub async fn remove_from_group(&self, group: &str, email: &str) -> Result<bool> {
		let done = sqlx::query("DELETE FROM group_members WHERE grp = ? AND email = lower(?)")
			.bind(group)
			.bind(email)
			.execute(&self.pool)
			.await
			.wrap_err("failed to remove from group")?;
		Ok(done.rows_affected() > 0)
	}

	pub async fn group_members(&self, group: &str) -> Result<Vec<String>> {
		let rows = sqlx::query("SELECT email FROM group_members WHERE grp = ?")
			.bind(group)
			.fetch_all(&self.pool)
			.await
			.wrap_err("failed to list group")?;
		Ok(rows.iter().map(|r| r.get("email")).collect())
	}

	pub async fn groups_of(&self, email: &str) -> Result<Vec<String>> {
		let rows = sqlx::query("SELECT grp FROM group_members WHERE email = lower(?) ORDER BY grp")
			.bind(email)
			.fetch_all(&self.pool)
			.await
			.wrap_err("failed to list groups")?;
		Ok(rows.iter().map(|r| r.get("grp")).collect())
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
fn user_from_row(r: &sqlx::sqlite::SqliteRow) -> User {
	User {
		id: r.get("id"),
		email: r.get("email"),
		username: r.get("username"),
		display_name: none_if_empty(r.get("display_name")),
		avatar_url: none_if_empty(r.get("avatar_url")),
	}
}

fn none_if_empty(s: String) -> Option<String> {
	if s.is_empty() { None } else { Some(s) }
}

fn expires_at_hours(hours: u32) -> String {
	(Timestamp::now() + (hours as i64).hours()).strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn expires_at_minutes(minutes: u32) -> String {
	(Timestamp::now() + (minutes as i64).minutes()).strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

impl std::fmt::Debug for Database {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Database").finish()
	}
}
