#![feature(default_field_values)]
//! The `va_access` cookie valeratrades.com sets on `.valeratrades.com`: an EdDSA JWT that the
//! site mints at sign-in and every service on a subdomain verifies locally, with the site's public key.

pub use jsonwebtoken::errors::Error;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

pub const COOKIE: &str = "va_access";
const ISSUER: &str = "https://valeratrades.com";
const AUDIENCE: &str = "va_sso";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Claims {
	/// the site's user id
	pub sub: String,
	pub email: String,
	pub username: String,
	pub admin: bool,
	pub groups: Vec<String>,
	/// unix seconds
	pub exp: i64,
}

impl Claims {
	/// Admins pass every group check.
	pub fn member_of(&self, group: &str) -> bool {
		self.admin || self.groups.iter().any(|g| g == group)
	}
}

pub fn mint(signing_key_pem: &str, claims: Claims) -> Result<String, Error> {
	let key = EncodingKey::from_ed_pem(signing_key_pem.as_bytes())?;
	let wire = Wire {
		claims,
		iss: ISSUER.into(),
		aud: AUDIENCE.into(),
	};
	jsonwebtoken::encode(&Header::new(Algorithm::EdDSA), &wire, &key)
}
pub struct Verifier {
	key: DecodingKey,
	validation: Validation,
}
impl Verifier {
	pub fn try_new(public_key_pem: &str) -> Result<Self, Error> {
		let mut validation = Validation::new(Algorithm::EdDSA);
		validation.set_issuer(&[ISSUER]);
		validation.set_audience(&[AUDIENCE]);
		validation.set_required_spec_claims(&["exp", "iss", "aud"]);
		Ok(Self {
			key: DecodingKey::from_ed_pem(public_key_pem.as_bytes())?,
			validation,
		})
	}

	pub fn verify(&self, token: &str) -> Result<Claims, Error> {
		Ok(jsonwebtoken::decode::<Wire>(token, &self.key, &self.validation)?.claims.claims)
	}
}

#[derive(Deserialize, Serialize)]
struct Wire {
	#[serde(flatten)]
	claims: Claims,
	iss: String,
	aud: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	const PRIVATE: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIA3bBKSXvm87i5bc706Y1QG1uj5EmbgUZygHJGfO1XYj\n-----END PRIVATE KEY-----\n";
	const PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAws8sYuYGZt4/OjCm05rzUQYOTAWBxVHPL1Fdg74KyV4=\n-----END PUBLIC KEY-----\n";

	fn claims(exp: i64) -> Claims {
		Claims {
			sub: "u1".into(),
			email: "a@example.com".into(),
			username: "a".into(),
			admin: false,
			groups: vec!["service-arb".into()],
			exp,
		}
	}

	fn now() -> i64 {
		std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
	}

	#[test]
	fn round_trip_and_rejections() {
		let v = Verifier::try_new(PUBLIC).unwrap();
		let c = claims(now() + 900);
		assert_eq!(v.verify(&mint(PRIVATE, c.clone()).unwrap()).unwrap(), c);
		assert!(v.verify(&mint(PRIVATE, claims(now() - 3600)).unwrap()).is_err(), "expired");
		let hs = jsonwebtoken::encode(&Header::default(), &c, &EncodingKey::from_secret(b"x")).unwrap();
		assert!(v.verify(&hs).is_err(), "not EdDSA");
		let other = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIHz7B0H6oZ1y3Yb8hB5o2c0X9m1wV6o1b8wXcY0f3s1a\n-----END PRIVATE KEY-----\n";
		assert!(v.verify(&mint(other, c).unwrap()).is_err(), "another key");
	}
}
