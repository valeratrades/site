use clap::Parser;
use site::conf::*;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[clap(flatten)]
	settings: SettingsFlags,
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
	use std::path::Path;

	use axum::Router;
	use leptos::prelude::*;
	use leptos_axum::*;
	use site::{app::*, auth::Database, blog, conf::Settings};
	use tower_http::services::ServeDir;
	use tracing::info;

	// Initialize global executor for any_spawner
	let _ = any_spawner::Executor::init_tokio();

	v_utils::clientside!();
	let cli = Cli::parse();
	let settings = Settings::try_build(cli.settings).unwrap();

	// Initialize database and run migrations
	let db = Database::new(&settings.clickhouse);
	if let Err(e) = db.migrate().await {
		tracing::warn!("Database migration failed (ClickHouse may not be running): {}", e);
	}

	// Warn about missing configurations
	if !settings.google_oauth.is_configured() {
		tracing::warn!("Google OAuth is not configured. Add [google_oauth] section with client_id and client_secret to enable Google sign-in.");
	}
	if settings.smtp.username.is_empty() {
		tracing::warn!("SMTP is not configured. Email verification will be skipped in development mode.");
	}

	let conf = get_configuration(Some("Cargo.toml")).unwrap();
	let addr = conf.leptos_options.site_addr;
	let leptos_options = conf.leptos_options;

	// Compile blog posts from .typ files to HTML
	let blog_source_dir = Path::new("public/blog");
	let blog_output_dir = Path::new("target/site/blog");
	blog::compile::init_blog_posts(blog_source_dir, blog_output_dir);

	// Build the router with server functions
	let leptos_options_clone = leptos_options.clone();
	let settings_clone = settings.clone();
	let app = Router::new()
		// Serve static blog HTML files at /YYYY/MM/DD/slug.html
		.nest_service("/20", ServeDir::new("target/site/blog/20"))
		.leptos_routes_with_context(
			&leptos_options,
			generate_route_list(App),
			move || {
				provide_context(settings_clone.clone());
			},
			{
				let leptos_options = leptos_options_clone.clone();
				move || shell(leptos_options.clone())
			},
		)
		.fallback(file_and_error_handler(move |_| {
			provide_context(settings.clone());
			shell(leptos_options_clone.clone())
		}))
		.with_state(leptos_options);

	// run our app with hyper (`axum::Server` is a re-export of `hyper::Server`)
	let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
	{
		let msg = format!("listening on http://{}", &addr);
		println!("{}", msg);
		info!("{}", msg);
	}
	axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
	// hydration is bootstrapped in [./lib.rs]
	panic!("not the correct access point");
}
