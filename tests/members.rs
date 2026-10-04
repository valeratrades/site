//! `/auth/members` against a real database and config: who may list and change a group, what an
//! entry says about the person's account, and that a change reaches the next `va_access`.

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
async fn admins_keep_a_group_and_its_members_sign_in_with_it() {
	let dir = std::env::temp_dir().join(format!("site-members-test-{}", std::process::id()));
	std::fs::create_dir_all(&dir).unwrap();
	std::env::set_var("XDG_STATE_HOME", &dir);
	let config = dir.join("site.toml");
	std::fs::write(
		&config,
		format!("site_url = \"http://localhost:61156\"\n[sso]\nadmins = [\"Root@x.com\"]\nsigning_key_pem = \"\"\"\n{PRIVATE}\"\"\"\n"),
	)
	.unwrap();
	let cli = Cli::parse_from(["site", "--config", config.to_str().unwrap()]);
	let live = LiveSettings::new(cli.settings, Duration::from_secs(60)).unwrap();
	let db = Database::try_new().await.unwrap();
	for (id, email, username) in [("m", "member@x.com", "alice"), ("f", "friend@x.com", "bob"), ("u", "unverified@x.com", "squatter")] {
		db.create_user(id, email, username, "pw").await.unwrap();
		db.create_session(&format!("session-{id}"), id, 1).await.unwrap();
	}
	db.mark_email_verified("m").await.unwrap();
	db.mark_email_verified("f").await.unwrap();

	let app = axum::Router::new()
		.route(
			"/auth/members",
			axum::routing::get(site::auth::sso::members)
				.put(site::auth::sso::add_member)
				.delete(site::auth::sso::remove_member),
		)
		.route("/auth/refresh", axum::routing::get(site::auth::sso::refresh))
		.with_state((live, db));
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let base = format!("http://{}", listener.local_addr().unwrap());
	tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
	let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
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
	let root = || Some(access("root@x.com", true));
	let call = |method: reqwest::Method, cookie: Option<String>, query: &[(&str, &str)]| {
		let mut req = http.request(method, format!("{base}/auth/members")).query(query);
		if let Some(c) = cookie {
			req = req.header("cookie", c);
		}
		req.send()
	};
	let list = |cookie| async move {
		let res = call(reqwest::Method::GET, cookie, &[("group", "service-arb")]).await.unwrap();
		assert_eq!(res.status(), 200);
		res.json::<serde_json::Value>().await.unwrap()
	};
	let groups_of = |session: &'static str| {
		let http = http.clone();
		let base = base.clone();
		async move {
			let res = http
				.get(format!("{base}/auth/refresh"))
				.query(&[("return_to", "http://localhost:59110/")])
				.header("cookie", format!("session_id={session}"))
				.send()
				.await
				.unwrap();
			let set = res.headers()["set-cookie"].to_str().unwrap();
			let jwt = set.strip_prefix("va_access=").unwrap().split(';').next().unwrap();
			va_sso::Verifier::try_from_signing_key(PRIVATE).unwrap().verify(jwt).unwrap().groups
		}
	};

	assert_eq!(list(root()).await, serde_json::json!([{"email": "root@x.com", "username": null, "display_name": null}]), "admins are in every group");
	assert!(groups_of("session-f").await.is_empty());

	for email in ["Member@x.com", "friend@x.com", "unverified@x.com", "never@x.com"] {
		assert_eq!(call(reqwest::Method::PUT, root(), &[("group", "service-arb"), ("email", email)]).await.unwrap().status(), 204);
	}
	assert_eq!(
		list(root()).await,
		serde_json::json!([
			{"email": "friend@x.com", "username": "bob", "display_name": null},
			{"email": "member@x.com", "username": "alice", "display_name": null},
			{"email": "never@x.com", "username": null, "display_name": null},
			{"email": "root@x.com", "username": null, "display_name": null},
			{"email": "unverified@x.com", "username": null, "display_name": null},
		]),
		"an unverified account could be anyone's, so its username is not the member's"
	);
	assert_eq!(groups_of("session-f").await, vec!["service-arb".to_string()], "added, the next sign-in carries the group");
	assert!(groups_of("session-u").await.is_empty(), "an unverified email carries no group");

	assert_eq!(call(reqwest::Method::DELETE, root(), &[("group", "service-arb"), ("email", "FRIEND@x.com")]).await.unwrap().status(), 204);
	assert!(groups_of("session-f").await.is_empty());
	assert_eq!(call(reqwest::Method::DELETE, root(), &[("group", "service-arb"), ("email", "friend@x.com")]).await.unwrap().status(), 404);

	let member = || Some(access("member@x.com", false));
	assert_eq!(call(reqwest::Method::GET, member(), &[("group", "service-arb")]).await.unwrap().status(), 403);
	assert_eq!(call(reqwest::Method::PUT, member(), &[("group", "service-arb"), ("email", "z@x.com")]).await.unwrap().status(), 403);
	assert_eq!(call(reqwest::Method::GET, None, &[("group", "service-arb")]).await.unwrap().status(), 401);
	assert_eq!(call(reqwest::Method::PUT, root(), &[("group", "admin"), ("email", "z@x.com")]).await.unwrap().status(), 400, "admins are the config's");
	assert_eq!(call(reqwest::Method::PUT, root(), &[("group", "service-arb"), ("email", "nope")]).await.unwrap().status(), 400);
	std::fs::remove_dir_all(&dir).unwrap();
}
