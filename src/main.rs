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
	use leptos_axum::{generate_route_list, LeptosRoutes};
	use site::{app::*, conf::Settings};
	use tracing::{debug, info};

	v_utils::clientside!();
	let cli = Cli::parse();
	let settings = Settings::try_build(cli.settings).unwrap();

	let conf = get_configuration(None).unwrap();
	let addr = conf.leptos_options.site_addr;
	let leptos_options = conf.leptos_options;

	let routes = generate_route_list(App);
	debug!(?routes);

	let app = Router::new()
		.leptos_routes_with_context(&leptos_options, routes, move || provide_context(settings.clone()), {
			let leptos_options = leptos_options.clone();
			move || shell(leptos_options.clone())
		})
		.fallback(leptos_axum::file_and_error_handler(shell))
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
