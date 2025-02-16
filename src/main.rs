use basic_nested_router::App;
use leptos::prelude::*;
use tracing::info;

fn main() {
	color_eyre::install().unwrap();
	//v_utils::utils::init_subscriber("/home/v/.local/state/site/.log".into()); // breaks the site for some reason
	_ = console_log::init_with_level(log::Level::Debug);
	console_error_panic_hook::set_once();

	info!("Starting app");
	mount_to_body(|| {
		view! { <App /> }
	})
}
