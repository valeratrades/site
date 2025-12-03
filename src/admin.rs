use leptos::{html::*, prelude::*};
use leptos_meta::{Title, TitleProps};

use crate::dashboards::{LoadingIndicator, LoadingIndicatorProps};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminData {
	pub creds: Option<Vec<(String, String)>>,
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

	// Convert HashMap<String, EnvString> to Vec<(String, String)> for serialization
	let creds = settings.admin.creds.map(|c| c.into_iter().map(|(k, v)| (k, v.0)).collect());

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
				Some(Ok(data)) => div()
					.child((
						h1().class("text-2xl font-bold mb-6").child("Admin Panel"),
						// Credentials section
						div().class("mb-8").child((
							h2().class("text-xl font-semibold mb-4 border-b pb-2").child("Credentials"),
							match data.creds {
								Some(creds) if !creds.is_empty() => {
									let yaml = creds.into_iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n");
									pre().class("bg-gray-800 rounded-lg p-4 font-mono text-sm overflow-x-auto").child(yaml).into_any()
								}
								_ => p().class("text-gray-500 italic").child("No credentials configured").into_any(),
							},
						)),
					))
					.into_any(),
				Some(Err(e)) => pre().class("text-red-500").child(format!("Error: {e}")).into_any(),
				None => LoadingIndicator(LoadingIndicatorProps { label: "Admin".into() }).into_any(),
			})
		}),
	})
}
