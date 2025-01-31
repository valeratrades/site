use leptos::prelude::*;
use leptos_routable::prelude::Routable;
use leptos_router::components::{A, Outlet};

use crate::AppRoutes;

#[derive(Routable)]
#[routes(view_prefix = "", view_suffix = "View", transition = false)]
pub enum AdminRoutes {
	#[route(path = "/")]
	AdminHome,

	#[route(path = "/users")]
	UserList,

	#[fallback]
	AdminNotFound,
}

#[component]
pub fn AdminHomeView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h2 class="text-2xl font-bold">"Admin Dashboard"</h2>
			<A
				href=AppRoutes::Admin(AdminRoutes::UserList)
				attr:class="inline-block px-4 py-2 mt-4 bg-blue-500 text-white rounded"
			>
				"Manage Users"
			</A>
		</div>
	}
}

#[component]
pub fn UserListView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h2 class="text-2xl font-bold">"User Management"</h2>
			<p>"List or manage users here."</p>
		</div>
	}
}

#[component]
pub fn AdminNotFoundView() -> impl IntoView {
	view! {
		<div class="p-4 text-center">
			<h2 class="text-2xl font-bold">"Admin 404: Not Found"</h2>
			<p>"This admin page doesn't exist."</p>
		</div>
	}
}

#[component]
pub fn AdminView() -> impl IntoView {
	view! { <Outlet /> }
}
