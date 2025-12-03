use leptos::{ev, html::*, prelude::*};
use leptos_meta::{Title, TitleProps};

use crate::dashboards::{LoadingIndicator, LoadingIndicatorProps};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminData {
	pub creds: Vec<(String, String)>,
}

#[server(GetAdminData)]
pub async fn get_admin_data() -> Result<AdminData, ServerFnError> {
	use crate::conf::Settings;

	// Get current user
	let user = crate::app::server_impl::get_current_user_impl().await?;
	let Some(user) = user else {
		return Err(ServerFnError::new("Not logged in"));
	};

	// Check if user is admin
	let settings = use_context::<Settings>().ok_or_else(|| ServerFnError::new("Settings not available"))?;
	if !settings.admin.users.contains(&user.username) {
		return Err(ServerFnError::new("Access denied: not an admin"));
	};

	let creds = settings.admin.creds.map(|c| c.into_iter().map(|(k, v)| (k, v.0)).collect()).unwrap_or_default();

	Ok(AdminData { creds })
}

#[component]
pub fn AdminView() -> impl IntoView {
	section().class("p-4 max-w-2xl mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Admin".into()),
		}),
		AdminContent(),
	))
}

#[island]
fn AdminContent() -> impl IntoView {
	let result_resource = Resource::new(|| (), |_| async move { get_admin_data().await });

	Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "Admin".into() }) }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || match result_resource.get() {
				Some(Ok(data)) =>
					if data.creds.is_empty() {
						p().class("text-gray-500 italic").child("No credentials configured").into_any()
					} else {
						div()
							.child((
								h2().class("text-xl font-semibold mb-4").child("Credentials"),
								div().class("space-y-3").child(
									data.creds
										.into_iter()
										.map(|(key, value)| CopyableCredential(CopyableCredentialProps { key, value }))
										.collect::<Vec<_>>(),
								),
							))
							.into_any()
					},
				Some(Err(e)) => pre().class("text-red-500").child(format!("Error: {e}")).into_any(),
				None => LoadingIndicator(LoadingIndicatorProps { label: "Admin".into() }).into_any(),
			})
		}),
	})
}

#[component]
fn CopyableCredential(key: String, value: String) -> impl IntoView {
	let copied = RwSignal::new(false);
	let value_for_click = value.clone();

	let on_copy = move |_| {
		let val = value_for_click.clone();
		copied.set(true);
		wasm_bindgen_futures::spawn_local(async move {
			if let Some(window) = web_sys::window() {
				let clipboard = window.navigator().clipboard();
				let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&val)).await;
			}
		});
		set_timeout(move || copied.set(false), std::time::Duration::from_secs(2));
	};

	div().class("flex items-center gap-3").child((
		span().class("text-gray-400 min-w-32 text-right").child(format!("{}:", key)),
		div()
			.class("flex-1 flex items-center gap-2 bg-gray-800 rounded px-3 py-2 font-mono text-sm cursor-pointer hover:bg-gray-700 transition-colors")
			.on(ev::click, on_copy)
			.child((
				span().class("flex-1 truncate").child(value),
				span().class("text-xs text-gray-500").child(move || if copied.get() { "Copied!" } else { "Click to copy" }),
			)),
	))
}
