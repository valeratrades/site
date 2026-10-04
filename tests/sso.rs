//! `/auth/refresh` against a real database and config: what the `va_access` cookie it sets
//! says, and where it sends a browser.

use std::time::Duration;

use clap::Parser;
use site::{
	auth::Database,
	config::{LiveSettings, SettingsFlags},
};

const PRIVATE: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIA3bBKSXvm87i5bc706Y1QG1uj5EmbgUZygHJGfO1XYj\n-----END PRIVATE KEY-----\n";
const PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAws8sYuYGZt4/OjCm05rzUQYOTAWBxVHPL1Fdg74KyV4=\n-----END PUBLIC KEY-----\n";

#[derive(Parser)]
struct Cli {
	#[clap(flatten)]
	settings: SettingsFlags,
}

#[tokio::test]
async fn refresh_signs_in_verified_members_and_returns_only_to_cookie_hosts() {
	let dir = std::env::temp_dir().join(format!("site-sso-test-{}", std::process::id()));
	std::fs::create_dir_all(&dir).unwrap();
	std::env::set_var("XDG_STATE_HOME", &dir);
	let config = dir.join("site.toml");
	std::fs::write(
		&config,
		format!(
			"site_url = \"http://localhost:61156\"\n[sso]\nadmins = [\"root@x.com\"]\nsigning_key_pem = \"\"\"\n{PRIVATE}\"\"\"\n"
		),
	)
	.unwrap();
	let cli = Cli::parse_from(["site", "--config", config.to_str().unwrap()]);
	let live = LiveSettings::new(cli.settings, Duration::from_secs(60)).unwrap();
	let db = Database::try_new().await.unwrap();
	for (id, email) in [("m", "member@x.com"), ("u", "unverified@x.com"), ("r", "root@x.com")] {
		db.create_user(id, email, id, "pw").await.unwrap();
		db.create_session(&format!("session-{id}"), id, 1).await.unwrap();
	}
	db.mark_email_verified("m").await.unwrap();
	db.mark_email_verified("r").await.unwrap();
	for email in ["Member@x.com", "unverified@x.com"] {
		db.add_to_group("service-arb", email, "root@x.com").await.unwrap();
	}

	let app = axum::Router::new().route("/auth/refresh", axum::routing::get(site::auth::sso::refresh).with_state((live, db)));
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let base = format!("http://{}", listener.local_addr().unwrap());
	tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
	let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
	let back = "http://localhost:59110/me?x=1";
	let refresh = |session: Option<&str>, to: &str| {
		let mut req = http.get(format!("{base}/auth/refresh")).query(&[("return_to", to)]);
		if let Some(s) = session {
			req = req.header("cookie", format!("session_id={s}"));
		}
		req.send()
	};
	let verifier = va_sso::Verifier::try_new(PUBLIC).unwrap();
	let claims_of = |res: &reqwest::Response| {
		let set = res.headers()["set-cookie"].to_str().unwrap();
		let jwt = set.strip_prefix("va_access=").unwrap().split(';').next().unwrap();
		verifier.verify(jwt).unwrap()
	};

	let res = refresh(Some("session-m"), back).await.unwrap();
	assert_eq!(res.status(), 303);
	assert_eq!(res.headers()["location"], back);
	let claims = claims_of(&res);
	assert_eq!((claims.email.as_str(), claims.admin, claims.member_of("service-arb")), ("member@x.com", false, true));

	let root = claims_of(&refresh(Some("session-r"), back).await.unwrap());
	assert!(root.admin && root.member_of("service-arb"), "admins pass every group check");

	let unverified = claims_of(&refresh(Some("session-u"), back).await.unwrap());
	assert!(!unverified.member_of("service-arb"), "an unverified email could be anyone's");

	let res = refresh(None, back).await.unwrap();
	let to = res.headers()["location"].to_str().unwrap();
	assert!(to.starts_with("/login?redirect_to=%2Fauth%2Frefresh%3Freturn_to%3D"), "{to}");

	assert_eq!(refresh(Some("session-m"), "https://evil.example/").await.unwrap().status(), 400);
	std::fs::remove_dir_all(&dir).unwrap();
}
