//! valeratrades.com as the sign-in for its subdomains: the `va_access` cookie ([`va_sso`]) set
//! beside `session_id`, and `/auth/refresh`, where a service sends a browser whose cookie is
//! missing or expired.

use axum::{
	extract::{Query, State},
	http::{HeaderMap, HeaderValue, StatusCode, header},
	response::{IntoResponse, Redirect, Response},
};

use super::{Database, User};
use crate::config::{LiveSettings, Settings, SsoConf};

const TTL_SECS: i64 = 15 * 60; // how stale a group or admin claim may get

/// `Set-Cookie` for `va_access`; `None` without `sso` configured. Groups and admin come only
/// with a verified email: an unverified one could be anyone's.
pub async fn access_cookie(settings: &Settings, db: &Database, user: &User) -> color_eyre::Result<Option<HeaderValue>> {
	let Some(sso) = &settings.sso else { return Ok(None) };
	let email = user.email.to_lowercase();
	let groups: Vec<String> = match db.is_email_verified(&user.id).await? {
		true => settings
			.groups
			.iter()
			.filter(|(_, emails)| emails.iter().any(|e| e.to_lowercase() == email))
			.map(|(g, _)| g.clone())
			.collect(),
		false => vec![],
	};
	let claims = va_sso::Claims {
		sub: user.id.clone(),
		email,
		username: user.username.clone(),
		admin: groups.iter().any(|g| g == "admin"),
		groups,
		exp: jiff::Timestamp::now().as_second() + TTL_SECS,
	};
	let jwt = va_sso::mint(&sso.signing_key_pem, claims)?;
	Ok(Some(cookie(settings, sso, &format!("{}={jwt}; Max-Age={TTL_SECS}", va_sso::COOKIE))))
}

/// `Set-Cookie` that removes `va_access`; `None` without `sso` configured.
pub fn cleared_access_cookie(settings: &Settings) -> Option<HeaderValue> {
	settings.sso.as_ref().map(|sso| cookie(settings, sso, &format!("{}=; Max-Age=0", va_sso::COOKIE)))
}

/// `; Secure` when the site is served over https.
pub fn secure(settings: &Settings) -> &'static str {
	match settings.site_url.starts_with("https://") {
		true => "; Secure",
		false => "",
	}
}
#[derive(serde::Deserialize)]
pub struct RefreshQuery {
	return_to: String,
}
/// Signed in: a fresh `va_access`, and back to `return_to`. Not: the login page, which comes back here.
pub async fn refresh(State((live, db)): State<(LiveSettings, Database)>, headers: HeaderMap, Query(q): Query<RefreshQuery>) -> Response {
	let settings = live.config().expect("the config loaded at start");
	let Some(sso) = &settings.sso else {
		return (StatusCode::NOT_FOUND, "sign-in for other services is not configured here").into_response();
	};
	if !returns_to_cookie_host(&settings, sso, &q.return_to) {
		return (StatusCode::BAD_REQUEST, format!("{} does not receive this site's sign-in cookie", q.return_to)).into_response();
	}
	let session = headers
		.get_all(header::COOKIE)
		.iter()
		.filter_map(|v| v.to_str().ok())
		.flat_map(|v| v.split(';'))
		.find_map(|c| c.trim().strip_prefix("session_id="));
	let user = match session {
		Some(s) => db.get_session_user(s).await.expect("the session table is readable"),
		None => None,
	};
	let Some(user) = user else {
		let back = format!("/auth/refresh?return_to={}", urlencoding(&q.return_to));
		return Redirect::to(&format!("/login?redirect_to={}", urlencoding(&back))).into_response();
	};
	let cookie = access_cookie(&settings, &db, &user)
		.await
		.expect("sso.signing_key_pem is an Ed25519 PKCS#8 PEM")
		.expect("sso is configured");
	([(header::SET_COOKIE, cookie)], Redirect::to(&q.return_to)).into_response()
}
fn cookie(settings: &Settings, sso: &SsoConf, value: &str) -> HeaderValue {
	let domain = sso.cookie_domain.as_ref().map(|d| format!("; Domain={d}")).unwrap_or_default();
	HeaderValue::from_str(&format!("{value}; Path=/; HttpOnly; SameSite=Lax{domain}{}", secure(settings))).expect("a JWT and a domain are header-safe")
}

fn returns_to_cookie_host(settings: &Settings, sso: &SsoConf, return_to: &str) -> bool {
	let Ok(to) = url::Url::parse(return_to) else { return false };
	let Some(host) = to.host_str() else { return false };
	match &sso.cookie_domain {
		Some(d) => to.scheme() == "https" && (host == d || host.ends_with(&format!(".{d}"))),
		None => url::Url::parse(&settings.site_url).expect("site_url is a URL").host_str() == Some(host),
	}
}

fn urlencoding(s: &str) -> String {
	url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
