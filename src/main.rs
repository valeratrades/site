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
	use axum::Router;
	use leptos::prelude::*;
	use leptos_axum::*;
	use site::{app::*, conf::Settings};
	use tracing::info;

	// Initialize global executor for any_spawner
	let _ = any_spawner::Executor::init_tokio();

	v_utils::clientside!();
	let cli = Cli::parse();
	let settings = Settings::try_build(cli.settings).unwrap();

	let conf = get_configuration(None).unwrap();
	let addr = conf.leptos_options.site_addr;
	let leptos_options = conf.leptos_options;

	// Simplified setup for now - routes will be handled by Leptos
	let leptos_options_clone = leptos_options.clone();
	let app = Router::new()
		.fallback(file_and_error_handler(move |_| {
			provide_context(settings.clone());
			shell(leptos_options_clone.clone())
		}))
		.with_state(leptos_options);

	// run our app with hyper (`axum::Server` is a re-export of `hyper::Server`)
	let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
	info!("listening on http://{}", &addr);
	axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
	// hydration is bootstrapped in [./lib.rs]
	panic!("not the correct access point");
}
