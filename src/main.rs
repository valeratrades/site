use std::time::Duration;

use clap::Parser;
use site::config::*;

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

	use axum::{
		Router,
		extract::Path as AxumPath,
		http::StatusCode,
		response::{Html, IntoResponse},
	};
	use leptos::prelude::*;
	use leptos_axum::*;
	use site::{app::*, auth::Database, blog, config::LiveSettings};
	use tracing::info;

	async fn serve_blog_post(AxumPath((year, month, day, slug)): AxumPath<(String, String, String, String)>) -> impl IntoResponse {
		let file_path = format!("target/site/blog/{}/{}/{}/{}", year, month, day, slug);
		match tokio::fs::read_to_string(&file_path).await {
			Ok(content) => {
				// Get the post title from slug (remove .html extension)
				let slug_without_ext = slug.trim_end_matches(".html");
				let title = blog::compile::get_post_title(slug_without_ext).unwrap_or_else(|| slug_without_ext.to_string());

				// Inject title and centering CSS into the <head>
				let with_title = content.replace("<head>", &format!("<head>\n<title>{}</title>", title));
				let styled = with_title.replace(
					"</head>",
					r#"<style>
html {
  display: flex;
  justify-content: center;
}
body {
  max-width: 42rem;
  width: 100%;
  padding: 1rem;
}
img {
  max-width: 100%;
  height: auto;
}
figure {
  margin: 1rem 0;
}
</style>
</head>"#,
				);
				Html(styled).into_response()
			}
			Err(_) => (StatusCode::NOT_FOUND, "Post not found").into_response(),
		}
	}

	async fn serve_blog_year(AxumPath(year): AxumPath<i32>) -> impl IntoResponse {
		Html(blog::render_blog_list_html(Some(year), None, None))
	}

	async fn serve_blog_year_month(AxumPath((year, month)): AxumPath<(i32, u32)>) -> impl IntoResponse {
		Html(blog::render_blog_list_html(Some(year), Some(month), None))
	}

	async fn serve_blog_year_month_day(AxumPath((year, month, day)): AxumPath<(i32, u32, u32)>) -> impl IntoResponse {
		Html(blog::render_blog_list_html(Some(year), Some(month), Some(day)))
	}

	// Initialize global executor for any_spawner
	let _ = any_spawner::Executor::init_tokio();

	v_utils::clientside!();
	let cli = Cli::parse();
	// LiveSettings provides hot-reload: config file changes are picked up automatically
	let live_settings = LiveSettings::new(cli.settings, Duration::from_secs(5)).unwrap();
	let settings = live_settings.initial();

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
	let live_settings_clone = live_settings.clone();

	async fn serve_blog_index() -> impl IntoResponse {
		Html(blog::render_blog_list_html(None, None, None))
	}

	// Blog routes (with and without trailing slash)
	let blog_router = Router::new()
		.route("/", axum::routing::get(serve_blog_index))
		.route("/{year}", axum::routing::get(serve_blog_year))
		.route("/{year}/", axum::routing::get(serve_blog_year))
		.route("/{year}/{month}", axum::routing::get(serve_blog_year_month))
		.route("/{year}/{month}/", axum::routing::get(serve_blog_year_month))
		.route("/{year}/{month}/{day}", axum::routing::get(serve_blog_year_month_day))
		.route("/{year}/{month}/{day}/", axum::routing::get(serve_blog_year_month_day))
		.route("/{year}/{month}/{day}/{slug}", axum::routing::get(serve_blog_post));

	let app = Router::new()
		.leptos_routes_with_context(
			&leptos_options,
			generate_route_list(App),
			move || {
				provide_context(live_settings_clone.clone());
			},
			{
				let leptos_options = leptos_options_clone.clone();
				move || shell(leptos_options.clone())
			},
		)
		// Blog routes (listings and posts)
		.nest("/blog", blog_router)
		.fallback(file_and_error_handler(move |_| {
			provide_context(live_settings.clone());
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
