#![feature(duration_constructors)]
mod admin;
mod dashboards;
pub(crate) mod utils;
use leptos::prelude::*;
use leptos_meta::{Html, Meta, Title};
use leptos_routable::prelude::{combine_paths, Routable};
use leptos_router::components::{Router, A};

use crate::{
	admin::{AdminRoutes, AdminView},
	dashboards::DashboardsView,
};

#[derive(Clone)]
pub struct AuthContext {
	pub is_logged_in: ReadSignal<bool>,
	pub set_logged_in: WriteSignal<bool>,
}

#[component]
pub fn AuthProvider(children: Children) -> impl IntoView {
	let (is_logged_in, set_logged_in) = signal(false);
	provide_context(AuthContext { is_logged_in, set_logged_in });
	children()
}

fn get_auth_redirect_path() -> AppRoutes {
	AppRoutes::Login
}

fn get_auth_condition() -> Option<bool> {
	Some(expect_context::<AuthContext>().is_logged_in.get())
}

#[derive(Routable)]
#[routes(view_prefix = "", view_suffix = "View", transition = false)]
pub enum AppRoutes {
	#[route(path = "/")]
	Home,
	#[parent_route(path = "/dashboards", ssr = "::leptos_router::SsrMode::default()")]
	Dashboards(dashboards::Routes),
	#[route(path = "/asset")]
	AssetList,
	#[protected_route(path = "/profile", condition = "get_auth_condition", redirect_path = "get_auth_redirect_path", fallback = "NotFoundView")]
	Profile,
	#[route(path = "/login")]
	Login,
	#[protected_parent_route(
		path = "/admin",
		condition = "get_auth_condition",
		redirect_path = "get_auth_redirect_path",
		fallback = "NotFoundView",
		ssr = "::leptos_router::SsrMode::default()"
	)]
	Admin(AdminRoutes),
	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

#[component]
pub fn LoginView() -> impl IntoView {
	let auth = expect_context::<AuthContext>();
	let navigate = leptos_router::hooks::use_navigate();
	let login = Callback::<()>::new(move |_| {
		auth.set_logged_in.set(true);
		navigate(&AppRoutes::Profile.to_string(), Default::default());
	});
	let logout = Callback::<()>::new(move |_| {
		auth.set_logged_in.set(false);
	});
	view! {
		<div class="p-4 text-center">
			<h1 class="text-2xl font-bold mb-4">"Login"</h1>
			<Show
				when=auth.is_logged_in
				fallback=move || {
						view! {
							<div class="space-y-4">
								<p class="text-gray-600">"You need to login to access protected routes."</p>
								<button class="px-4 py-2 bg-green-500 text-white rounded" on:click=move |_| login.run(())>
									"Login"
								</button>
							</div>
						}
				}
			>
				<div class="space-y-4">
					<p class="text-green-600">"You are logged in!"</p>
					<button class="px-4 py-2 bg-red-500 text-white rounded" on:click=move |_| logout.run(())>
						"Logout"
					</button>
				</div>
			</Show>
			<div class="mt-4">
				<A href=AppRoutes::Home attr:class="inline-block px-4 py-2 bg-blue-500 text-white rounded">
					"Back Home"
				</A>
			</div>
		</div>
	}
}

#[component]
pub fn HomeView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h1 class="text-2xl font-bold">"Welcome Home!"</h1>
			<p>"Explore the site using the navigation links below."</p>
		</div>
	}
}

#[component]
pub fn AssetListView() -> impl IntoView {
	use leptos_router::hooks::use_location;
	view! {
		<div class="p-4">
			<h1 class="text-2xl font-bold mb-4">"Asset List"</h1>
			<div class="space-y-4">
				<h2 class="text-xl">"Test Navigation Links"</h2>
				<div class="flex flex-col space-y-2">
					<A href=AppRoutes::Home attr:class="inline-block px-4 py-2 bg-green-500 text-white rounded">
						"→ Go Home"
					</A>
					<A href=AppRoutes::Profile attr:class="inline-block px-4 py-2 bg-blue-500 text-white rounded">
						"→ Profile Page"
					</A>
					<A
						href=AppRoutes::Dashboards(dashboards::Routes::Home)
						attr:class="inline-block px-4 py-2 bg-blue-500 text-white rounded"
					>
						{format!("→ Dashboards Home: {}", AppRoutes::Dashboards(dashboards::Routes::Home))}
					</A>
					<A
						href=AppRoutes::Admin(AdminRoutes::AdminHome)
						attr:class="inline-block px-4 py-2 bg-blue-500 text-white rounded"
					>
						"→ Admin Dashboard"
					</A>
					<A href=AppRoutes::NotFound attr:class="inline-block px-4 py-2 bg-blue-500 text-white rounded">
						"→ 404 Page"
					</A>
				</div>
				<div class="mt-4 p-4 rounded">
					<p class="text-sm font-mono">"Current Path: " {move || use_location().pathname}</p>
					<p class="text-sm font-mono">"Query String: " {move || use_location().search}</p>
				</div>
			</div>
		</div>
	}
}

#[component]
pub fn ProfileView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h1 class="text-2xl font-bold">"User Profile"</h1>
			<p>"Name: John Doe"</p>
			<p>"Membership: Gold"</p>
			<p>"Email: john.doe@example.com"</p>
			<A href=AppRoutes::Home attr:class="inline-block px-4 py-2 mt-4 bg-green-500 text-white rounded">
				"Back Home"
			</A>
		</div>
	}
}

#[component]
pub fn NotFoundView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h1 class="text-2xl font-bold">"404: Not Found"</h1>
			<p>"Sorry, we can't find that page."</p>
			<A href=AppRoutes::Home attr:class="inline-block px-4 py-2 bg-green-500 text-white rounded mt-4">
				"Go Home"
			</A>
		</div>
	}
}

#[component]
pub fn App() -> impl IntoView {
	leptos_meta::provide_meta_context();
	view! {
		<Html attr:lang="en" attr:dir="ltr" />
		<Title text="My sity-site" />
		<Meta charset="UTF-8" />
		<Meta name="viewport" content="width=device-width, initial-scale=1.0" />
		<main class="min-h-screen">
			<Router>
				<nav class="flex space-x-4 p-4 bg-gray-900 text-white">
					<A href=AppRoutes::Home attr:class="text-white px-3 py-1 bg-green-600 rounded">
						"Home"
					</A>
					<A href=AppRoutes::Dashboards(dashboards::Routes::Home) attr:class="text-white px-3 py-1 bg-blue-600 rounded">
						"Dashboards"
					</A>
					<A href=AppRoutes::AssetList attr:class="text-white px-3 py-1 bg-blue-600 rounded">
						"Assets"
					</A>
					<A href=AppRoutes::Profile attr:class="text-white px-3 py-1 bg-blue-600 rounded">
						"Profile"
					</A>
					<A href=AppRoutes::Admin(AdminRoutes::AdminHome) attr:class="text-white px-3 py-1 bg-blue-600 rounded">
						"Admin"
					</A>
				</nav>
				<AuthProvider>{move || AppRoutes::routes()}</AuthProvider>
			</Router>
		</main>
	}
}
