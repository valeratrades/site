//! Global keyboard handlers
//!
//! Provides helpers for setting up keyboard event handlers.

/// Hook to register an escape handler that fires on document-level Escape key press.
#[cfg(not(feature = "ssr"))]
pub fn use_escape(handler: impl Fn() + 'static) {
	use wasm_bindgen::{JsCast, closure::Closure};

	if let Some(window) = web_sys::window() {
		let closure = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
			if e.key() == "Escape" {
				handler();
			}
		});
		window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
		closure.forget();
	}
}

#[cfg(feature = "ssr")]
pub fn use_escape(_handler: impl Fn() + 'static) {}

/// Hook to register a key handler that fires on document-level key press.
/// Skips events when an input/textarea is focused.
/// The handler receives the key name.
#[cfg(not(feature = "ssr"))]
pub fn use_key(handler: impl Fn(&str) + 'static) {
	use wasm_bindgen::{JsCast, closure::Closure};

	if let Some(window) = web_sys::window() {
		let closure = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
			// Skip if focus is on an input element
			if let Some(document) = web_sys::window().and_then(|w| w.document()) {
				if let Some(active) = document.active_element() {
					let tag = active.tag_name().to_uppercase();
					if tag == "INPUT" || tag == "TEXTAREA" {
						return;
					}
				}
			}
			handler(&e.key());
		});

		window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
		closure.forget();
	}
}

#[cfg(feature = "ssr")]
pub fn use_key(_handler: impl Fn(&str) + 'static) {}
