use leptos::{html::*, prelude::*};
use leptos_meta::{Title, TitleProps};

use crate::dashboards::{LoadingIndicator, LoadingIndicatorProps};

#[server(SlowHelloWorld)]
pub async fn slow_hello_world() -> Result<String, ServerFnError> {
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

	// Slow delay
	tokio::time::sleep(std::time::Duration::from_secs(15)).await;

	Ok("Hello World!".to_string())
}

#[component]
pub fn TmpView() -> impl IntoView {
	section().class("p-4 max-w-md mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Tmp".into()),
		}),
		TmpContent(),
	))
}

#[island]
fn TmpContent() -> impl IntoView {
	let result_resource = Resource::new(|| (), |_| async move { slow_hello_world().await });

	Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "Tmp".into() }) }.into(),
		children: ToChildren::to_children(move || {
			IntoRender::into_render(move || match result_resource.get() {
				Some(Ok(msg)) => pre().child(msg).into_any(),
				Some(Err(e)) => pre().class("text-red-500").child(format!("Error: {e}")).into_any(),
				None => LoadingIndicator(LoadingIndicatorProps { label: "Tmp".into() }).into_any(),
			})
		}),
	})
}
