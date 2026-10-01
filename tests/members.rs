//! `/auth/members` against a real database and config: who may list a group, and what an
//! entry says about the person's account.

use std::time::Duration;

use clap::Parser;
use site::{
	auth::Database,
	config::{LiveSettings, SettingsFlags},
};

const PRIVATE: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIA3bBKSXvm87i5bc706Y1QG1uj5EmbgUZygHJGfO1XYj\n-----END PRIVATE KEY-----\n";

#[derive(Parser)]
struct Cli {
	#[clap(flatten)]
	settings: SettingsFlags,
}

#[tokio::test]
async fn admins_list_a_group_with_the_verified_accounts_in_it() {
	let dir = std::env::temp_dir().join(format!("site-members-test-{}", std::process::id()));
	std::fs::create_dir_all(&dir).unwrap();
	std::env::set_var("XDG_STATE_HOME", &dir);
	let config = dir.join("site.toml");
	std::fs::write(
		&config,
		format!(
			"site_url = \"http://localhost:61156\"\n[sso]\nsigning_key_pem = \"\"\"\n{PRIVATE}\"\"\"\n[groups]\nservice-arb = [\"Member@x.com\", \"unverified@x.com\", \"never@x.com\"]\nadmin = [\"root@x.com\"]\n"
		),
	)
	.unwrap();
	let cli = Cli::parse_from(["site", "--config", config.to_str().unwrap()]);
	let live = LiveSettings::new(cli.settings, Duration::from_secs(60)).unwrap();
	let db = Database::try_new().await.unwrap();
	db.create_user("m", "member@x.com", "alice", "pw").await.unwrap();
	db.mark_email_verified("m").await.unwrap();
	db.create_user("u", "unverified@x.com", "squatter", "pw").await.unwrap();

	let app = axum::Router::new().route("/auth/members", axum::routing::get(site::auth::sso::members).with_state((live, db)));
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let base = format!("http://{}", listener.local_addr().unwrap());
	tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
	let http = reqwest::Client::new();
	let access = |email: &str, admin: bool| {
		let claims = va_sso::Claims {
			sub: email.into(),
			email: email.into(),
			username: email.into(),
			admin,
			groups: vec![],
			exp: jiff::Timestamp::now().as_second() + 900,
		};
		format!("{}={}", va_sso::COOKIE, va_sso::mint(PRIVATE, claims).unwrap())
	};
	let list = |cookie: Option<String>, group: &str| {
		let mut req = http.get(format!("{base}/auth/members")).query(&[("group", group)]);
		if let Some(c) = cookie {
			req = req.header("cookie", c);
		}
		req.send()
	};

	let res = list(Some(access("root@x.com", true)), "service-arb").await.unwrap();
	assert_eq!(res.status(), 200);
	let body: serde_json::Value = res.json().await.unwrap();
	assert_eq!(
		body,
		serde_json::json!([
			{"email": "member@x.com", "username": "alice", "display_name": null},
			{"email": "never@x.com", "username": null, "display_name": null},
			{"email": "root@x.com", "username": null, "display_name": null},
			{"email": "unverified@x.com", "username": null, "display_name": null},
		]),
		"an unverified account could be anyone's, so its username is not the member's"
	);

	assert_eq!(list(Some(access("member@x.com", false)), "service-arb").await.unwrap().status(), 403);
	assert_eq!(list(None, "service-arb").await.unwrap().status(), 401);
	assert_eq!(list(Some(access("root@x.com", true)), "nope").await.unwrap().status(), 404);
	std::fs::remove_dir_all(&dir).unwrap();
}
