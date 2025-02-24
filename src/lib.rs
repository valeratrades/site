#![feature(stmt_expr_attributes)]
#![feature(duration_constructors)]
pub mod app;
pub mod conf;
pub(crate) mod dashboards;
pub(crate) mod utils;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	#[allow(unused_imports)] // In `islands` mode clippy will mark `use app::*` as `unused`, but it still is.
	use app::*;

	console_error_panic_hook::set_once();
	leptos::mount::hydrate_islands();
}
