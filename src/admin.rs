use leptos::{ev, html::*, prelude::*};
use leptos_meta::{Title, TitleProps};
use wasm_bindgen::JsCast;

use crate::dashboards::{LoadingIndicator, LoadingIndicatorProps};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminData {
	pub creds: Vec<(String, String)>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminFileInfo {
	pub id: String,
	pub filename: String,
}

#[server(GetAdminData)]
pub async fn get_admin_data() -> Result<AdminData, ServerFnError> {
	use crate::config::LiveSettings;

	// Get current user
	let user = crate::app::server_impl::get_current_user_impl().await?;
	let Some(user) = user else {
		return Err(ServerFnError::new("Not logged in"));
	};

	// Check if user is admin and get their permission level
	let settings = use_context::<LiveSettings>().map(|ls| ls.config()).ok_or_else(|| ServerFnError::new("Settings not available"))?;
	let user_permission = settings.admin.users.get(&user.username).ok_or_else(|| ServerFnError::new("Access denied: not an admin"))?;
	let user_level: f64 = **user_permission;

	// Filter credentials by permission level
	let creds: Vec<(String, String)> = settings
		.admin
		.creds
		.map(|levels| {
			levels
				.into_iter()
				.filter_map(|(level_str, creds_map)| {
					let required_level: f64 = level_str.parse().ok()?;
					if user_level * 100.0 >= required_level {
						Some(creds_map.into_iter().map(|(k, v)| (k, v.0)).collect::<Vec<_>>())
					} else {
						None
					}
				})
				.flatten()
				.collect()
		})
		.unwrap_or_default();

	Ok(AdminData { creds })
}

#[server(ListAdminFiles)]
pub async fn list_admin_files() -> Result<Vec<AdminFileInfo>, ServerFnError> {
	use crate::{auth::Database, config::LiveSettings};

	// Get current user
	let user = crate::app::server_impl::get_current_user_impl().await?;
	let Some(user) = user else {
		return Err(ServerFnError::new("Not logged in"));
	};

	// Check if user is admin
	let settings = use_context::<LiveSettings>().map(|ls| ls.config()).ok_or_else(|| ServerFnError::new("Settings not available"))?;
	if !settings.admin.users.contains_key(&user.username) {
		return Err(ServerFnError::new("Access denied: not an admin"));
	};

	let db = Database::new(&settings.clickhouse);
	let files = db.list_admin_files().await.map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

	Ok(files.into_iter().map(|f| AdminFileInfo { id: f.id, filename: f.filename }).collect())
}

#[server(UploadAdminFile)]
pub async fn upload_admin_file(filename: String, content_type: String, data_base64: String) -> Result<(), ServerFnError> {
	use crate::{auth::Database, config::LiveSettings};

	// Get current user
	let user = crate::app::server_impl::get_current_user_impl().await?;
	let Some(user) = user else {
		return Err(ServerFnError::new("Not logged in"));
	};

	// Check if user is admin
	let settings = use_context::<LiveSettings>().map(|ls| ls.config()).ok_or_else(|| ServerFnError::new("Settings not available"))?;
	if !settings.admin.users.contains_key(&user.username) {
		return Err(ServerFnError::new("Access denied: not an admin"));
	};

	let id = uuid::Uuid::new_v4().to_string();
	let db = Database::new(&settings.clickhouse);
	db.create_admin_file(&id, &filename, &content_type, &data_base64, &user.username)
		.await
		.map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

	Ok(())
}

#[server(DownloadAdminFile)]
pub async fn download_admin_file(id: String) -> Result<(String, String, String), ServerFnError> {
	use crate::{auth::Database, config::LiveSettings};

	// Get current user
	let user = crate::app::server_impl::get_current_user_impl().await?;
	let Some(user) = user else {
		return Err(ServerFnError::new("Not logged in"));
	};

	// Check if user is admin
	let settings = use_context::<LiveSettings>().map(|ls| ls.config()).ok_or_else(|| ServerFnError::new("Settings not available"))?;
	if !settings.admin.users.contains_key(&user.username) {
		return Err(ServerFnError::new("Access denied: not an admin"));
	};

	let db = Database::new(&settings.clickhouse);
	let file = db.get_admin_file(&id).await.map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

	match file {
		Some(f) => Ok((f.filename, f.content_type, f.data)),
		None => Err(ServerFnError::new("File not found")),
	}
}

#[server(DeleteAdminFile)]
pub async fn delete_admin_file(id: String) -> Result<(), ServerFnError> {
	use crate::{auth::Database, config::LiveSettings};

	// Get current user
	let user = crate::app::server_impl::get_current_user_impl().await?;
	let Some(user) = user else {
		return Err(ServerFnError::new("Not logged in"));
	};

	// Check if user is admin
	let settings = use_context::<LiveSettings>().map(|ls| ls.config()).ok_or_else(|| ServerFnError::new("Settings not available"))?;
	if !settings.admin.users.contains_key(&user.username) {
		return Err(ServerFnError::new("Access denied: not an admin"));
	};

	let db = Database::new(&settings.clickhouse);
	db.delete_admin_file(&id).await.map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

	Ok(())
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
					.class("space-y-8")
					.child((CredentialsSection(CredentialsSectionProps { creds: data.creds }), AdminFilesSection()))
					.into_any(),
				Some(Err(e)) => pre().class("text-red-500").child(format!("Error: {e}")).into_any(),
				None => LoadingIndicator(LoadingIndicatorProps { label: "Admin".into() }).into_any(),
			})
		}),
	})
}

#[component]
fn CredentialsSection(creds: Vec<(String, String)>) -> impl IntoView {
	if creds.is_empty() {
		div().into_any()
	} else {
		div()
			.child((
				h2().child("Credentials"),
				div().class("space-y-3").child(
					creds
						.into_iter()
						.map(|(key, value)| CopyableCredential(CopyableCredentialProps { key, value }))
						.collect::<Vec<_>>(),
				),
			))
			.into_any()
	}
}

#[island]
fn AdminFilesSection() -> impl IntoView {
	let files_resource = Resource::new(|| (), |_| async move { list_admin_files().await });
	let uploading = RwSignal::new(false);
	let error_msg = RwSignal::new(Option::<String>::None);

	let on_file_selected = move |_| {
		wasm_bindgen_futures::spawn_local(async move {
			let document = web_sys::window().unwrap().document().unwrap();
			let input: web_sys::HtmlInputElement = document.get_element_by_id("file-upload-input").unwrap().dyn_into().unwrap();

			if let Some(file_list) = input.files() {
				let count = file_list.length();
				if count == 0 {
					return;
				}

				uploading.set(true);
				error_msg.set(None);

				// Upload all selected files
				for i in 0..count {
					if let Some(file) = file_list.get(i) {
						let file: web_sys::File = file;
						let filename = file.name();
						let content_type = file.type_();

						let array_buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await;
						match array_buffer {
							Ok(ab) => {
								let uint8_array = js_sys::Uint8Array::new(&ab);
								let mut bytes = vec![0u8; uint8_array.length() as usize];
								uint8_array.copy_to(&mut bytes);

								use base64::{Engine, engine::general_purpose::STANDARD};
								let data_base64 = STANDARD.encode(&bytes);

								if let Err(e) = upload_admin_file(filename.clone(), content_type, data_base64).await {
									error_msg.set(Some(format!("Failed to upload {}: {e}", filename)));
									break;
								}
							}
							Err(_) => {
								error_msg.set(Some(format!("Failed to read {}", filename)));
								break;
							}
						}
					}
				}

				files_resource.refetch();
				input.set_value("");
				uploading.set(false);
			}
		});
	};

	let on_download = move |id: String, filename: String| {
		wasm_bindgen_futures::spawn_local(async move {
			match download_admin_file(id).await {
				Ok((fname, content_type, data_base64)) => {
					use base64::{Engine, engine::general_purpose::STANDARD};
					if let Ok(bytes) = STANDARD.decode(&data_base64) {
						let array = js_sys::Uint8Array::from(bytes.as_slice());
						let blob_parts = js_sys::Array::new();
						blob_parts.push(&array);

						let options = web_sys::BlobPropertyBag::new();
						options.set_type(&content_type);

						if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options) {
							if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
								let document = web_sys::window().unwrap().document().unwrap();
								let a: web_sys::HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();
								a.set_href(&url);
								a.set_download(&fname);
								a.click();
								let _ = web_sys::Url::revoke_object_url(&url);
							}
						}
					}
				}
				Err(e) => {
					web_sys::window().unwrap().alert_with_message(&format!("Download failed: {e}")).ok();
				}
			}
		});
		let _ = filename;
	};

	let on_delete = move |id: String| {
		wasm_bindgen_futures::spawn_local(async move {
			if let Err(e) = delete_admin_file(id).await {
				web_sys::window().unwrap().alert_with_message(&format!("Delete failed: {e}")).ok();
			} else {
				files_resource.refetch();
			}
		});
	};

	div().class("mt-6").child((
		div().class("flex items-center justify-between mb-3").child((
			h2().child("Files"),
			// Upload button (hidden input + styled button)
			div().child((
				input()
					.id("file-upload-input")
					.attr("type", "file")
					.attr("multiple", "true")
					.attr("style", "display:none")
					.on(ev::change, on_file_selected),
				button()
					.class("px-2 py-1 text-xs bg-green-600 hover:bg-green-500 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed")
					.prop("disabled", move || uploading.get())
					.on(ev::click, move |_| {
						let document = web_sys::window().unwrap().document().unwrap();
						let input: web_sys::HtmlInputElement = document.get_element_by_id("file-upload-input").unwrap().dyn_into().unwrap();
						input.click();
					})
					.child(move || if uploading.get() { "Uploading..." } else { "Upload" }),
			)),
		)),
		div().class("border border-gray-700/50 bg-gray-900/30 rounded-lg p-4").child((
			// Scrollable file list
			div().class("max-h-64 overflow-y-auto").child(move || match files_resource.get() {
				Some(Ok(files)) =>
					if files.is_empty() {
						p().class("text-gray-500 italic text-sm").child("No files uploaded yet").into_any()
					} else {
						div()
							.class("space-y-2")
							.child(
								files
									.into_iter()
									.map(|f| {
										let id_for_download = f.id.clone();
										let id_for_delete = f.id.clone();
										let filename_for_download = f.filename.clone();
										div()
											.class("flex items-center justify-between gap-2 p-2 bg-gray-800/50 rounded hover:bg-gray-800 transition-colors")
											.child((
												span().class("flex-1 truncate text-sm").child(f.filename),
												div().class("flex gap-2").child((
													button()
														.class("px-2 py-1 text-xs bg-blue-600 hover:bg-blue-500 rounded transition-colors")
														.on(ev::click, {
															let fname = filename_for_download.clone();
															move |_| on_download(id_for_download.clone(), fname.clone())
														})
														.child("Download"),
													button()
														.class("px-2 py-1 text-xs bg-red-600/80 hover:bg-red-500 rounded transition-colors")
														.on(ev::click, move |_| on_delete(id_for_delete.clone()))
														.child("Delete"),
												)),
											))
									})
									.collect::<Vec<_>>(),
							)
							.into_any()
					},
				Some(Err(e)) => p().class("text-red-500 text-sm").child(format!("Error: {e}")).into_any(),
				None => p().class("text-gray-500 text-sm").child("Loading...").into_any(),
			}),
			// Error message
			move || error_msg.get().map(|msg| p().class("text-red-500 text-sm mt-2").child(msg)),
		)),
	))
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
