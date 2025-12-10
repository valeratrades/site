use leptos::{ev, html::*, prelude::*, svg::svg};
use leptos_meta::{MetaTags, Stylesheet, StylesheetProps, Title, TitleProps, provide_meta_context};
use leptos_routable::prelude::*;
use leptos_router::{
	components::{A, AProps, Router},
	hooks::use_location,
};

use crate::{
	admin::AdminView,
	auth::User,
	blog::{self, BlogView},
	dashboards::{self, DashboardsView},
};

// Server functions for authentication
#[cfg(feature = "ssr")]
pub mod server_impl {
	use leptos::server_fn::error::ServerFnError;
	use tracing::{error, info, instrument};

	use super::*;
	use crate::{
		auth::{Database, EmailSender},
		config::{LiveSettings, Settings},
	};

	fn get_settings() -> Result<Settings, ServerFnError> {
		use_context::<LiveSettings>().map(|ls| ls.config()).ok_or_else(|| ServerFnError::new("Settings not available"))
	}

	fn get_db() -> Result<Database, ServerFnError> {
		let settings = get_settings()?;
		Ok(Database::new(&settings.clickhouse))
	}

	#[instrument(skip(password), fields(email = %email, username = %username))]
	pub async fn register_impl(email: String, username: String, password: String) -> Result<String, ServerFnError> {
		info!("Registration attempt");
		let settings = get_settings()?;
		let db = Database::new(&settings.clickhouse);

		if db.email_exists(&email).await.map_err(|e| {
			error!("Database error checking email: {}", e);
			ServerFnError::new(format!("Database error: {}", e))
		})? {
			info!("Email already registered");
			return Err(ServerFnError::new("Email already registered"));
		}

		let user_id = uuid::Uuid::new_v4().to_string();
		db.create_user(&user_id, &email, &username, &password).await.map_err(|e| {
			error!("Failed to create user: {}", e);
			ServerFnError::new(format!("Failed to create user: {}", e))
		})?;

		// Create verification token and send email
		let token = uuid::Uuid::new_v4().to_string();
		db.create_email_token(&token, &user_id, 24).await.map_err(|e| {
			error!("Failed to create verification token: {}", e);
			ServerFnError::new(format!("Failed to create verification token: {}", e))
		})?;

		// Send verification email
		if !settings.smtp.username.is_empty() {
			let email_sender = EmailSender::new(&settings.smtp).map_err(|e| {
				error!("Email configuration error: {}", e);
				ServerFnError::new(format!("Email configuration error: {}", e))
			})?;

			let verification_link = format!("{}/verify?token={}", settings.site_url, token);
			email_sender.send_verification_email(&email, &username, &verification_link).await.map_err(|e| {
				error!("Failed to send verification email: {}", e);
				ServerFnError::new(format!("Failed to send verification email: {}", e))
			})?;

			info!("Registration successful, verification email sent");
			Ok("Please check your email to verify your account".to_string())
		} else {
			// SMTP not configured, auto-verify for dev
			db.mark_email_verified(&user_id).await.map_err(|e| {
				error!("Failed to mark email verified: {}", e);
				ServerFnError::new(format!("Failed to verify email: {}", e))
			})?;
			info!("Registration successful (SMTP not configured, auto-verified)");
			Ok("Account created (email verification skipped - SMTP not configured)".to_string())
		}
	}

	#[instrument(skip(token))]
	pub async fn verify_email_impl(token: String) -> Result<(), ServerFnError> {
		let db = get_db()?;

		let user_id = db
			.verify_email_token(&token)
			.await
			.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
			.ok_or_else(|| ServerFnError::new("Invalid or expired verification token"))?;

		db.mark_email_verified(&user_id).await.map_err(|e| ServerFnError::new(format!("Failed to verify email: {}", e)))?;

		db.delete_email_token(&token).await.map_err(|e| ServerFnError::new(format!("Failed to delete token: {}", e)))?;

		Ok(())
	}

	#[instrument(skip(password), fields(email_or_username = %email_or_username))]
	pub async fn login_impl(email_or_username: String, password: String) -> Result<User, ServerFnError> {
		info!("Login attempt");
		let settings = get_settings()?;
		let db = get_db()?;

		// Try email first, then username
		let user_result = db.get_user_by_email(&email_or_username).await.map_err(|e| {
			error!("Database error during login: {}", e);
			ServerFnError::new(format!("Database error: {}", e))
		})?;

		let (user, password_hash) = match user_result {
			Some(result) => result,
			None => {
				// Try username if email lookup failed
				db.get_user_by_username(&email_or_username)
					.await
					.map_err(|e| {
						error!("Database error during login: {}", e);
						ServerFnError::new(format!("Database error: {}", e))
					})?
					.ok_or_else(|| {
						info!("No account found for email or username");
						ServerFnError::new("No account found with this email or username")
					})?
			}
		};

		if !bcrypt::verify(&password, &password_hash).unwrap_or(false) {
			info!("Incorrect password");
			return Err(ServerFnError::new("Incorrect password"));
		}

		// Check email verification (skip if SMTP not configured)
		let smtp_configured = !settings.smtp.username.is_empty();
		if smtp_configured
			&& !db.is_email_verified(&user.id).await.map_err(|e| {
				error!("Database error checking email verification: {}", e);
				ServerFnError::new(format!("Database error: {}", e))
			})? {
			info!("Email not verified");
			return Err(ServerFnError::new("Please verify your email before logging in"));
		}

		// Create session
		let session_id = uuid::Uuid::new_v4().to_string();
		db.create_session(&session_id, &user.id, 24 * 7).await.map_err(|e| {
			error!("Failed to create session: {}", e);
			ServerFnError::new(format!("Failed to create session: {}", e))
		})?;

		// Set cookie via response header
		use leptos_axum::ResponseOptions;
		if let Some(response) = use_context::<ResponseOptions>() {
			response.insert_header(
				axum::http::header::SET_COOKIE,
				axum::http::HeaderValue::from_str(&format!(
					"session_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
					session_id,
					60 * 60 * 24 * 7 // 1 week in seconds
				))
				.unwrap(),
			);
		}

		info!("Login successful");
		Ok(user)
	}

	pub async fn get_current_user_impl() -> Result<Option<User>, ServerFnError> {
		use axum::http::header::COOKIE;
		use leptos_axum::extract;

		let headers: axum::http::HeaderMap = extract().await.map_err(|e| ServerFnError::new(format!("Failed to extract headers: {}", e)))?;

		let session_id = headers.get(COOKIE).and_then(|v| v.to_str().ok()).and_then(|cookies| {
			cookies.split(';').find_map(|cookie| {
				let cookie = cookie.trim();
				if cookie.starts_with("session_id=") {
					Some(cookie.trim_start_matches("session_id=").to_string())
				} else {
					None
				}
			})
		});

		let Some(session_id) = session_id else {
			return Ok(None);
		};

		let db = get_db()?;

		db.get_session_user(&session_id).await.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))
	}

	pub async fn logout_impl() -> Result<(), ServerFnError> {
		use axum::http::header::COOKIE;
		use leptos_axum::{ResponseOptions, extract};

		let headers: axum::http::HeaderMap = extract().await.map_err(|e| ServerFnError::new(format!("Failed to extract headers: {}", e)))?;

		let session_id = headers.get(COOKIE).and_then(|v| v.to_str().ok()).and_then(|cookies| {
			cookies.split(';').find_map(|cookie| {
				let cookie = cookie.trim();
				if cookie.starts_with("session_id=") {
					Some(cookie.trim_start_matches("session_id=").to_string())
				} else {
					None
				}
			})
		});

		if let Some(session_id) = session_id {
			let db = get_db()?;
			let _ = db.delete_session(&session_id).await;
		}

		// Clear cookie
		if let Some(response) = use_context::<ResponseOptions>() {
			response.insert_header(
				axum::http::header::SET_COOKIE,
				axum::http::HeaderValue::from_str("session_id=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0").unwrap(),
			);
		}

		Ok(())
	}

	pub async fn google_auth_start_impl() -> Result<String, ServerFnError> {
		use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, basic::BasicClient};

		let settings = get_settings()?;
		if !settings.google_oauth.is_configured() {
			return Err(ServerFnError::new("Google OAuth is not configured"));
		}

		let db = get_db()?;

		let client = BasicClient::new(ClientId::new(settings.google_oauth.client_id.clone()))
			.set_client_secret(ClientSecret::new(settings.google_oauth.client_secret.clone()))
			.set_auth_uri(AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap())
			.set_redirect_uri(RedirectUrl::new(format!("{}/auth/google/callback", settings.site_url)).unwrap());

		let csrf_token = CsrfToken::new_random();
		let state = csrf_token.secret().clone();

		// Store the state for verification
		db.create_oauth_state(&state, 10) // 10 minutes expiry
			.await
			.map_err(|e| ServerFnError::new(format!("Failed to store OAuth state: {}", e)))?;

		let (auth_url, _) = client
			.authorize_url(|| csrf_token)
			.add_scope(Scope::new("openid".to_string()))
			.add_scope(Scope::new("email".to_string()))
			.add_scope(Scope::new("profile".to_string()))
			.url();

		Ok(auth_url.to_string())
	}

	pub async fn google_auth_callback_impl(code: String, state: String) -> Result<User, ServerFnError> {
		use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, RedirectUrl, TokenResponse, TokenUrl, basic::BasicClient};

		let settings = get_settings()?;
		let db = get_db()?;

		// Verify state
		if !db.verify_oauth_state(&state).await.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))? {
			return Err(ServerFnError::new("Invalid or expired OAuth state"));
		}

		// Delete used state
		let _ = db.delete_oauth_state(&state).await;

		let client = BasicClient::new(ClientId::new(settings.google_oauth.client_id.clone()))
			.set_client_secret(ClientSecret::new(settings.google_oauth.client_secret.clone()))
			.set_auth_uri(AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap())
			.set_token_uri(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap())
			.set_redirect_uri(RedirectUrl::new(format!("{}/auth/google/callback", settings.site_url)).unwrap());

		// Create HTTP client for token exchange
		let http_client = reqwest::ClientBuilder::new()
			.redirect(reqwest::redirect::Policy::none())
			.build()
			.map_err(|e| ServerFnError::new(format!("Failed to create HTTP client: {}", e)))?;

		// Exchange code for token
		let token_result = client
			.exchange_code(AuthorizationCode::new(code))
			.request_async(&http_client)
			.await
			.map_err(|e| ServerFnError::new(format!("Failed to exchange code for token: {}", e)))?;

		let access_token = token_result.access_token().secret();

		// Fetch user info from Google
		let http_client = reqwest::Client::new();
		let user_info: GoogleUserInfo = http_client
			.get("https://www.googleapis.com/oauth2/v2/userinfo")
			.bearer_auth(access_token)
			.send()
			.await
			.map_err(|e| ServerFnError::new(format!("Failed to fetch user info: {}", e)))?
			.json()
			.await
			.map_err(|e| ServerFnError::new(format!("Failed to parse user info: {}", e)))?;

		// Check if user exists by Google ID
		let user = if let Some(user) = db.get_user_by_google_id(&user_info.id).await.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))? {
			user
		} else if let Some((existing_user, _)) = db.get_user_by_email(&user_info.email).await.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))? {
			// Link Google account to existing user
			db.link_google_to_user(&existing_user.id, &user_info.id)
				.await
				.map_err(|e| ServerFnError::new(format!("Failed to link Google account: {}", e)))?;
			existing_user
		} else {
			// Create new user
			let user_id = uuid::Uuid::new_v4().to_string();
			let username = user_info.name.unwrap_or_else(|| user_info.email.split('@').next().unwrap_or("user").to_string());
			db.create_google_user(&user_id, &user_info.email, &username, &user_info.id)
				.await
				.map_err(|e| ServerFnError::new(format!("Failed to create user: {}", e)))?;
			User {
				id: user_id,
				email: user_info.email,
				username,
			}
		};

		// Create session
		let session_id = uuid::Uuid::new_v4().to_string();
		db.create_session(&session_id, &user.id, 24 * 7)
			.await
			.map_err(|e| ServerFnError::new(format!("Failed to create session: {}", e)))?;

		// Set cookie
		use leptos_axum::ResponseOptions;
		if let Some(response) = use_context::<ResponseOptions>() {
			response.insert_header(
				axum::http::header::SET_COOKIE,
				axum::http::HeaderValue::from_str(&format!("session_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}", session_id, 60 * 60 * 24 * 7)).unwrap(),
			);
		}

		Ok(user)
	}

	#[derive(serde::Deserialize)]
	pub struct GoogleUserInfo {
		pub id: String,
		pub email: String,
		pub name: Option<String>,
	}

	pub fn is_google_oauth_configured_impl() -> bool {
		get_settings().map(|s| s.google_oauth.is_configured()).unwrap_or(false)
	}
}

#[server(RegisterUser)]
pub async fn register_user(email: String, username: String, password: String) -> Result<String, ServerFnError> {
	server_impl::register_impl(email, username, password).await
}

#[server(LoginUser)]
pub async fn login_user(email_or_username: String, password: String) -> Result<User, ServerFnError> {
	server_impl::login_impl(email_or_username, password).await
}

#[server(GetCurrentUser)]
pub async fn get_current_user() -> Result<Option<User>, ServerFnError> {
	server_impl::get_current_user_impl().await
}

#[server(LogoutUser)]
pub async fn logout_user() -> Result<(), ServerFnError> {
	server_impl::logout_impl().await
}

#[server(VerifyEmail)]
pub async fn verify_email(token: String) -> Result<(), ServerFnError> {
	server_impl::verify_email_impl(token).await
}

#[server(GoogleAuthStart)]
pub async fn google_auth_start() -> Result<String, ServerFnError> {
	server_impl::google_auth_start_impl().await
}

#[server(GoogleAuthCallback)]
pub async fn google_auth_callback(code: String, state: String) -> Result<User, ServerFnError> {
	server_impl::google_auth_callback_impl(code, state).await
}

#[server(IsGoogleOAuthConfigured)]
pub async fn is_google_oauth_configured() -> Result<bool, ServerFnError> {
	Ok(server_impl::is_google_oauth_configured_impl())
}

/// Navigation link that highlights when the current route matches (or is a child of) the href.
/// For the root path "/", uses exact matching.
#[component]
fn NavLink(href: &'static str, label: &'static str) -> impl IntoView {
	let location = use_location();

	let is_active = Memo::new(move |_| {
		let pathname = location.pathname.get();
		if href == "/" {
			pathname == "/"
		} else {
			pathname == href || pathname.starts_with(&format!("{}/", href))
		}
	});

	A(AProps {
		href: href.to_string(),
		children: Box::new(move || {
			span()
				.class(move || {
					if is_active.get() {
						"px-3 py-1 rounded bg-gray-700 text-white transition-colors"
					} else {
						"px-3 py-1 rounded hover:bg-gray-700/50 transition-colors"
					}
				})
				.child(label)
				.into_any()
		}),
		target: None,
		exact: false,
		strict_trailing_slash: false,
		scroll: true,
	})
}

#[component]
fn TopBar() -> impl IntoView {
	nav().class("flex items-center px-4 py-2 bg-gray-800 text-white").child((
		div().class("flex gap-2").child((
			NavLink(NavLinkProps {
				href: "/dashboards",
				label: "Dashboards",
			}),
			NavLink(NavLinkProps { href: "/blog", label: "Blog" }),
			NavLink(NavLinkProps {
				href: "/contacts",
				label: "Contacts",
			}),
		)),
		div().class("ml-auto").child(UserButton()),
	))
}

#[island]
fn UserButton() -> impl IntoView {
	let user_resource = LocalResource::new(get_current_user);

	move || {
		match user_resource.get() {
			None => {
				// Loading state
				div().class("w-8 h-8 rounded-full bg-gray-700 animate-pulse").into_any()
			}
			Some(Ok(Some(user))) => {
				// Logged in - show initial, link to profile
				let initial = user.initial().to_string();
				let colors = ["bg-blue-500", "bg-green-500", "bg-purple-500", "bg-pink-500", "bg-orange-500", "bg-teal-500"];
				let color_idx = user.username.bytes().map(|b| b as usize).sum::<usize>() % colors.len();
				let bg_color = colors[color_idx];

				a().attr("href", "/profile")
					.child(
						div()
							.class(format!(
								"w-8 h-8 rounded-full {} flex items-center justify-center font-bold text-white hover:opacity-80 transition-opacity",
								bg_color
							))
							.child(initial),
					)
					.into_any()
			}
			Some(Ok(None)) | Some(Err(_)) => {
				// Not logged in - show silhouette, link to profile (will redirect to login)
				a().attr("href", "/profile")
					.child(
						div()
							.class("w-8 h-8 rounded-full bg-gray-600 flex items-center justify-center hover:bg-gray-500 transition-colors")
							.child(
								svg()
									.attr("viewBox", "0 0 24 24")
									.attr("fill", "none")
									.attr("stroke", "currentColor")
									.attr("stroke-width", "2")
									.class("w-5 h-5")
									.child(leptos::svg::path().attr("d", "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z")),
							),
					)
					.into_any()
			}
		}
	}
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<AutoReload options=options.clone() />
				<HydrationScripts options islands=true />
				<MetaTags />
				// all embedded plotly plots assume this is in scope
				<script src="https://cdn.plot.ly/plotly-3.0.1.min.js"></script>
			</head>
			<body>
				<App />
			</body>
		</html>
	}
}

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();
	(
		Stylesheet(StylesheetProps {
			id: Some("leptos".to_owned()),
			href: format!("/pkg/{}.css", env!("CARGO_PKG_NAME")),
		}),
		Title(TitleProps {
			formatter: None,
			text: Some("My Site".into()),
		}),
		view! {
			<Router>
				<TopBar />
				<main class="min-h-screen">{move || AppRoutes::routes()}</main>
			</Router>
		},
	)
}

#[derive(Routable)]
#[routes(view_prefix = "", view_suffix = "View", transition = false)]
pub enum AppRoutes {
	#[route(path = "/")]
	Home,
	#[parent_route(path = "/dashboards")]
	Dashboards(dashboards::Routes),
	#[parent_route(path = "/blog")]
	Blog(blog::Routes),
	#[route(path = "/contacts")]
	Contacts,
	#[route(path = "/profile")]
	Profile,
	#[route(path = "/login")]
	Login,
	#[route(path = "/verify")]
	Verify,
	#[route(path = "/auth/google/callback")]
	GoogleCallback,
	#[route(path = "/tmp")]
	Tmp,
	#[route(path = "/admin")]
	Admin,
	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

/// Renders the home page - redirects to /dashboards
#[component]
fn HomeView() -> impl IntoView {
	// SSR redirect
	#[cfg(feature = "ssr")]
	{
		use leptos_axum::ResponseOptions;
		if let Some(response) = use_context::<ResponseOptions>() {
			response.set_status(axum::http::StatusCode::TEMPORARY_REDIRECT);
			response.insert_header(axum::http::header::LOCATION, axum::http::HeaderValue::from_static("/dashboards"));
		}
	}
	// Return minimal content (won't be seen due to redirect)
	()
}

#[component]
fn ContactsView() -> impl IntoView {
	section().class("p-8 max-w-2xl mx-auto").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Contacts".into()),
		}),
		h1().class("text-3xl font-bold mb-8 text-center").child("Get in Touch"),
		div().class("grid grid-cols-1 sm:grid-cols-2 gap-4").child((
			contact_link("GitHub", "https://github.com/valeratrades", "M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"),
			contact_link("LinkedIn", "https://linkedin.com/in/valeratrades", "M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"),
			contact_link("Discord", "https://discord.com/users/valeratrades", "M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9460 2.4189-2.1568 2.4189z"),
			contact_link("Twitter", "https://twitter.com/valeratrades", "M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"),
			contact_link("Telegram", "https://t.me/valeratrades", "M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"),
			contact_link("Email", "mailto:valeratrades@gmail.com", "M24 5.457v13.909c0 .904-.732 1.636-1.636 1.636h-3.819V11.73L12 16.64l-6.545-4.91v9.273H1.636A1.636 1.636 0 0 1 0 19.366V5.457c0-2.023 2.309-3.178 3.927-1.964L5.455 4.64 12 9.548l6.545-4.91 1.528-1.145C21.69 2.28 24 3.434 24 5.457z"),
		)),
	))
}

fn contact_link(name: &'static str, href: &'static str, svg_path: &'static str) -> impl IntoView {
	a().attr("href", href)
		.attr("target", "_blank")
		.attr("rel", "noopener noreferrer")
		.class("flex items-center gap-3 p-4 bg-gray-800 hover:bg-gray-700 rounded-lg transition-colors text-white")
		.child((
			svg()
				.attr("viewBox", "0 0 24 24")
				.attr("fill", "currentColor")
				.class("w-6 h-6")
				.child(leptos::svg::path().attr("d", svg_path)),
			span().class("font-medium").child(name),
		))
}

#[component]
fn LoginView() -> impl IntoView {
	section().class("p-4 max-w-md mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Login".into()),
		}),
		LoginForm(),
	))
}

#[island]
fn LoginForm() -> impl IntoView {
	let user_resource = LocalResource::new(get_current_user);
	let google_oauth_configured = LocalResource::new(is_google_oauth_configured);

	let email_or_username = RwSignal::new(String::new());
	let email = RwSignal::new(String::new());
	let username = RwSignal::new(String::new());
	let password = RwSignal::new(String::new());
	let error = RwSignal::new(Option::<String>::None);
	let success_message = RwSignal::new(Option::<String>::None);
	let is_register_mode = RwSignal::new(false);
	let is_loading = RwSignal::new(false);
	let google_loading = RwSignal::new(false);
	let redirect_to = RwSignal::new("/".to_string());

	// Get redirect_to from URL query params (only on client)
	Effect::new(move |_| {
		if let Some(window) = web_sys::window() {
			if let Ok(search) = window.location().search() {
				if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
					if let Some(redirect) = params.get("redirect_to") {
						redirect_to.set(redirect);
					}
				}
			}
		}
	});

	let on_submit = move |e: web_sys::SubmitEvent| {
		e.prevent_default();
		is_loading.set(true);
		error.set(None);
		success_message.set(None);

		let password_val = password.get();
		let redirect = redirect_to.get();

		if is_register_mode.get() {
			let email_val = email.get();
			let username_val = username.get();
			wasm_bindgen_futures::spawn_local(async move {
				match register_user(email_val, username_val, password_val).await {
					Ok(msg) => {
						success_message.set(Some(msg));
						is_loading.set(false);
					}
					Err(e) => {
						let msg = format!("{}", e);
						let clean_msg = if msg.contains("Email already registered") {
							"This email is already registered. Please login instead.".to_string()
						} else {
							// Show actual error for debugging
							format!("Registration failed: {}", msg)
						};
						error.set(Some(clean_msg));
						is_loading.set(false);
					}
				}
			});
		} else {
			let login_val = email_or_username.get();
			wasm_bindgen_futures::spawn_local(async move {
				match login_user(login_val, password_val).await {
					Ok(_) =>
						if let Some(window) = web_sys::window() {
							let _ = window.location().set_href(&redirect);
						},
					Err(e) => {
						// Extract clean error message from ServerFnError
						let msg = format!("{}", e);
						let clean_msg = if msg.contains("No account found") {
							"No account found with this email or username. Please register first.".to_string()
						} else if msg.contains("Incorrect password") {
							"Incorrect password".to_string()
						} else if msg.contains("verify your email") {
							"Please verify your email before logging in".to_string()
						} else {
							"Login failed. Please try again.".to_string()
						};
						error.set(Some(clean_msg));
						is_loading.set(false);
					}
				}
			});
		}
	};

	move || {
		match user_resource.get() {
			None => div().class("text-center").child("Loading...").into_any(),
			Some(Ok(Some(_user))) => {
				// Already logged in - redirect to destination
				Effect::new(move |_| {
					if let Some(window) = web_sys::window() {
						let _ = window.location().set_href(&redirect_to.get());
					}
				});
				div().class("text-center").child("Already logged in, redirecting...").into_any()
			}
			Some(Ok(None)) | Some(Err(_)) => {
				// Check for success message first
				if let Some(msg) = success_message.get() {
					return div()
						.class("text-center")
						.child((
							h1().class("text-2xl font-bold mb-4 text-green-600").child("Registration Successful"),
							div().class("bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4").child(msg),
							button()
								.attr("type", "button")
								.class("text-blue-500 hover:underline")
								.on(ev::click, move |_| {
									success_message.set(None);
									is_register_mode.set(false);
								})
								.child("Go to Login"),
						))
						.into_any();
				}

				// Login/Register form
				let form_title = move || if is_register_mode.get() { "Register" } else { "Login" };
				let toggle_text = move || {
					if is_register_mode.get() {
						"Already have an account? Login"
					} else {
						"Don't have an account? Register"
					}
				};

				form()
					.on(ev::submit, on_submit)
					.child((
						h1().class("text-2xl font-bold mb-6 text-center").child(form_title),
						// Error message
						move || error.get().map(|e| div().class("bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4").child(e)),
						// Login: Email or Username field; Register: Email and Username fields
						move || {
							if is_register_mode.get() {
								// Register mode: separate Email and Username fields
								div()
									.child((
										div().class("mb-4").child((
											label().class("block text-gray-700 text-sm font-bold mb-2").attr("for", "email").child("Email"),
											input()
												.attr("type", "email")
												.attr("id", "email")
												.attr("required", "")
												.attr("placeholder", "you@example.com")
												.class("w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:border-blue-500")
												.prop("value", move || email.get())
												.on(ev::input, move |e| {
													let val = event_target_value(&e);
													email.set(val);
												}),
										)),
										div().class("mb-4").child((
											label().class("block text-gray-700 text-sm font-bold mb-2").attr("for", "username").child("Username"),
											input()
												.attr("type", "text")
												.attr("id", "username")
												.attr("required", "")
												.attr("placeholder", "johndoe")
												.class("w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:border-blue-500")
												.prop("value", move || username.get())
												.on(ev::input, move |e| {
													let val = event_target_value(&e);
													username.set(val);
												}),
										)),
									))
									.into_any()
							} else {
								// Login mode: single Email or Username field
								div()
									.class("mb-4")
									.child((
										label()
											.class("block text-gray-700 text-sm font-bold mb-2")
											.attr("for", "email_or_username")
											.child("Email or Username"),
										input()
											.attr("type", "text")
											.attr("id", "email_or_username")
											.attr("required", "")
											.attr("placeholder", "you@example.com or johndoe")
											.class("w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:border-blue-500")
											.prop("value", move || email_or_username.get())
											.on(ev::input, move |e| {
												let val = event_target_value(&e);
												email_or_username.set(val);
											}),
									))
									.into_any()
							}
						},
						// Password field
						div().class("mb-6").child((
							label().class("block text-gray-700 text-sm font-bold mb-2").attr("for", "password").child("Password"),
							input()
								.attr("type", "password")
								.attr("id", "password")
								.attr("required", "")
								.attr("placeholder", "••••••••")
								.class("w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:border-blue-500")
								.prop("value", move || password.get())
								.on(ev::input, move |e| {
									let val = event_target_value(&e);
									password.set(val);
								}),
						)),
						// Submit button
						button()
							.attr("type", "submit")
							.attr("disabled", move || is_loading.get())
							.class("w-full bg-blue-500 text-white py-2 px-4 rounded hover:bg-blue-600 transition-colors disabled:opacity-50")
							.child(move || {
								if is_loading.get() {
									"Loading..."
								} else if is_register_mode.get() {
									"Register"
								} else {
									"Login"
								}
							}),
						// Toggle link
						div().class("mt-4 text-center").child(
							button()
								.attr("type", "button")
								.class("text-blue-500 hover:underline")
								.on(ev::click, move |_| {
									let switching_to_register = !is_register_mode.get();
									if switching_to_register {
										// If switching to register and the login field looks like an email, copy it
										let val = email_or_username.get();
										if val.contains('@') {
											email.set(val);
										}
									} else {
										// If switching to login and email is filled, copy it to login field
										let val = email.get();
										if !val.is_empty() {
											email_or_username.set(val);
										}
									}
									is_register_mode.set(switching_to_register);
								})
								.child(toggle_text),
						),
						// Google Sign-in button (only shown if configured and not in register mode)
						move || {
							let show_google = google_oauth_configured.get().map(|r| r.unwrap_or(false)).unwrap_or(false);
							if show_google && !is_register_mode.get() {
								Some(
									div().class("mt-6").child((
										div().class("relative flex items-center justify-center mb-4").child((
											div().class("flex-grow border-t border-gray-300"),
											span().class("px-3 text-gray-500 text-sm").child("or"),
											div().class("flex-grow border-t border-gray-300"),
										)),
										button()
											.attr("type", "button")
											.attr("disabled", move || google_loading.get())
											.class(
												"w-full flex items-center justify-center gap-3 bg-white border border-gray-300 text-gray-700 py-2 px-4 rounded hover:bg-gray-50 transition-colors disabled:opacity-50",
											)
											.on(ev::click, move |_| {
												google_loading.set(true);
												error.set(None);
												wasm_bindgen_futures::spawn_local(async move {
													match google_auth_start().await {
														Ok(url) =>
															if let Some(window) = web_sys::window() {
																let _ = window.location().set_href(&url);
															},
														Err(e) => {
															error.set(Some(e.to_string()));
															google_loading.set(false);
														}
													}
												});
											})
											.child((
												// Google icon
												svg().attr("viewBox", "0 0 24 24").class("w-5 h-5").child((
													leptos::svg::path().attr("fill", "#4285F4").attr(
														"d",
														"M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z",
													),
													leptos::svg::path().attr("fill", "#34A853").attr(
														"d",
														"M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z",
													),
													leptos::svg::path().attr("fill", "#FBBC05").attr(
														"d",
														"M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z",
													),
													leptos::svg::path().attr("fill", "#EA4335").attr(
														"d",
														"M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z",
													),
												)),
												span().child(move || if google_loading.get() { "Redirecting..." } else { "Sign in with Google" }),
											)),
									)),
								)
							} else {
								None
							}
						},
					))
					.into_any()
			}
		}
	}
}

#[component]
fn ProfileView() -> impl IntoView {
	section().class("p-4 max-w-md mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Profile".into()),
		}),
		ProfileContent(),
	))
}

#[island]
fn ProfileContent() -> impl IntoView {
	let user_resource = LocalResource::new(get_current_user);

	let on_logout = move |_| {
		wasm_bindgen_futures::spawn_local(async move {
			let _ = logout_user().await;
			if let Some(window) = web_sys::window() {
				let _ = window.location().set_href("/");
			}
		});
	};

	move || {
		match user_resource.get() {
			None => {
				// Loading
				div().class("text-center").child("Loading...").into_any()
			}
			Some(Ok(Some(user))) => {
				// Logged in - show profile
				div()
					.class("text-center")
					.child((
						h1().class("text-2xl font-bold mb-4").child("Profile"),
						div().class("bg-gray-100 rounded-lg p-6 mb-4").child((
							p().class("text-lg mb-2").child(format!("Username: {}", user.username)),
							p().class("text-gray-600").child(format!("Email: {}", user.email)),
						)),
						button()
							.class("w-full bg-red-500 text-white py-2 px-4 rounded hover:bg-red-600 transition-colors")
							.on(ev::click, on_logout)
							.child("Logout"),
					))
					.into_any()
			}
			Some(Ok(None)) | Some(Err(_)) => {
				// Not logged in - redirect to login with return URL
				Effect::new(move |_| {
					if let Some(window) = web_sys::window() {
						let _ = window.location().set_href("/login?redirect_to=/profile");
					}
				});
				div().class("text-center").child("Redirecting to login...").into_any()
			}
		}
	}
}

#[component]
fn VerifyView() -> impl IntoView {
	section().class("p-4 max-w-md mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Verify Email".into()),
		}),
		VerifyForm(),
	))
}

#[island]
fn VerifyForm() -> impl IntoView {
	let status = RwSignal::new(Option::<Result<(), String>>::None);
	let is_loading = RwSignal::new(true);

	// Get token from URL query params
	Effect::new(move |_| {
		if let Some(window) = web_sys::window() {
			if let Ok(search) = window.location().search() {
				let params = web_sys::UrlSearchParams::new_with_str(&search).ok();
				if let Some(token) = params.and_then(|p| p.get("token")) {
					wasm_bindgen_futures::spawn_local(async move {
						match verify_email(token).await {
							Ok(()) => {
								status.set(Some(Ok(())));
								is_loading.set(false);
							}
							Err(e) => {
								status.set(Some(Err(e.to_string())));
								is_loading.set(false);
							}
						}
					});
				} else {
					status.set(Some(Err("No verification token provided".to_string())));
					is_loading.set(false);
				}
			}
		}
	});

	move || {
		if is_loading.get() {
			div().class("text-center").child("Verifying your email...").into_any()
		} else {
			match status.get() {
				Some(Ok(())) => div()
					.class("text-center")
					.child((
						h1().class("text-2xl font-bold mb-4 text-green-600").child("Email Verified!"),
						p().class("mb-4").child("Your email has been successfully verified. You can now log in."),
						A(AProps {
							href: "/login".to_string(),
							children: Box::new(|| view! { "Go to Login" }.into_any()),
							target: None,
							exact: false,
							strict_trailing_slash: false,
							scroll: true,
						})
						.attr("class", "inline-block px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"),
					))
					.into_any(),
				Some(Err(e)) => div()
					.class("text-center")
					.child((
						h1().class("text-2xl font-bold mb-4 text-red-600").child("Verification Failed"),
						div().class("bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4").child(e),
						A(AProps {
							href: "/login".to_string(),
							children: Box::new(|| view! { "Go to Login" }.into_any()),
							target: None,
							exact: false,
							strict_trailing_slash: false,
							scroll: true,
						})
						.attr("class", "inline-block px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"),
					))
					.into_any(),
				None => div().class("text-center").child("Loading...").into_any(),
			}
		}
	}
}

#[component]
fn GoogleCallbackView() -> impl IntoView {
	section().class("p-4 max-w-md mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Google Sign-in".into()),
		}),
		GoogleCallbackHandler(),
	))
}

#[island]
fn GoogleCallbackHandler() -> impl IntoView {
	let status = RwSignal::new(Option::<Result<(), String>>::None);
	let is_loading = RwSignal::new(true);

	Effect::new(move |_| {
		if let Some(window) = web_sys::window() {
			if let Ok(search) = window.location().search() {
				let params = web_sys::UrlSearchParams::new_with_str(&search).ok();
				let code = params.as_ref().and_then(|p| p.get("code"));
				let state = params.as_ref().and_then(|p| p.get("state"));

				match (code, state) {
					(Some(code), Some(state)) => {
						wasm_bindgen_futures::spawn_local(async move {
							match google_auth_callback(code, state).await {
								Ok(_) => {
									// Redirect to home on success
									if let Some(window) = web_sys::window() {
										let _ = window.location().set_href("/");
									}
								}
								Err(e) => {
									status.set(Some(Err(e.to_string())));
									is_loading.set(false);
								}
							}
						});
					}
					_ => {
						status.set(Some(Err("Missing code or state parameter".to_string())));
						is_loading.set(false);
					}
				}
			}
		}
	});

	move || {
		if is_loading.get() {
			div()
				.class("text-center")
				.child((
					div().class("text-lg").child("Signing you in with Google..."),
					div().class("mt-4 animate-spin inline-block w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full"),
				))
				.into_any()
		} else if let Some(Err(e)) = status.get() {
			div()
				.class("text-center")
				.child((
					h1().class("text-2xl font-bold mb-4 text-red-600").child("Sign-in Failed"),
					div().class("bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4").child(e),
					A(AProps {
						href: "/login".to_string(),
						children: Box::new(|| view! { "Try Again" }.into_any()),
						target: None,
						exact: false,
						strict_trailing_slash: false,
						scroll: true,
					})
					.attr("class", "inline-block px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"),
				))
				.into_any()
		} else {
			div().class("text-center").child("Redirecting...").into_any()
		}
	}
}

#[component]
pub fn NotFoundView() -> impl IntoView {
	div().class("p-4 text-center").child((
		h1().class("text-2xl font-bold"),
		p().child("Sorry, we can't find that page"),
		A(AProps {
			href: "/".to_string(),
			children: Box::new(|| view! { "Go Home" }.into_any()),
			target: None,
			exact: false,
			strict_trailing_slash: false,
			scroll: true,
		})
		.attr("class", "inline-block px-4 py-2 bg-green-500 text-white rounded mt-4"),
	))
}

pub use crate::tmp::TmpView;
