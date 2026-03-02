use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
	pub id: String,
	pub email: String,
	pub username: String,
	pub display_name: Option<String>,
	pub avatar_url: Option<String>,
}

impl User {
	pub fn initial(&self) -> char {
		self.display_name
			.as_deref()
			.or(Some(&self.username))
			.and_then(|s| s.chars().next())
			.unwrap_or('?')
			.to_ascii_uppercase()
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoginCredentials {
	pub email_or_username: String,
	pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegisterCredentials {
	pub email: String,
	pub username: String,
	pub password: String,
}
