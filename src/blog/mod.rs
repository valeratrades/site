use leptos::{html::*, prelude::*};
use leptos_meta::{Title, TitleProps};
use leptos_routable::prelude::*;

#[cfg(feature = "ssr")]
pub mod compile;

#[derive(Routable)]
#[routes(transition = false)]
pub enum Routes {
	#[route(path = "/")]
	Home,

	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

#[component]
pub fn BlogView() -> impl IntoView {
	leptos_router::components::Outlet()
}

#[component]
fn NotFoundView() -> impl IntoView {
	section().class("p-4 text-center").child((
		h1().class("text-2xl font-bold").child("Post Not Found"),
		a().attr("href", "/blog")
			.class("inline-block px-4 py-2 bg-green-600 text-white rounded mt-2")
			.child("Back to Blog"),
	))
}

#[server(GetBlogPosts)]
pub async fn get_posts() -> Result<Vec<(String, String, String)>, ServerFnError> {
	use chrono::Datelike;
	// Returns Vec<(date_display, title, url)>
	let posts = compile::get_blog_posts();
	Ok(posts
		.iter()
		.map(|p| {
			let date_display = p.created.format("%b %d, %Y").to_string();
			let url = format!("/{}/{:02}/{:02}/{}.html", p.created.year(), p.created.month(), p.created.day(), p.slug);
			(date_display, p.title.clone(), url)
		})
		.collect())
}

// Matklad-style blog list
#[component]
fn HomeView() -> impl IntoView {
	section().class("max-w-2xl mx-auto px-4 py-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Blog".into()),
		}),
		BlogPostList(),
	))
}

#[island]
fn BlogPostList() -> impl IntoView {
	let posts = LocalResource::new(get_posts);

	move || match posts.get() {
		None => div().class("text-gray-500").child("Loading...").into_any(),
		Some(Ok(posts)) => ul()
			.class("list-none p-0 m-0")
			.child(
				posts
					.into_iter()
					.map(|(date, title, url)| {
						li().class("py-1").child((
							span().class("text-gray-500 tabular-nums").child(date),
							span().child(" "),
							a().attr("href", url).child(h2().class("inline text-base font-normal text-black hover:underline").child(title)),
							hr().class("border-gray-300 mt-1"),
						))
					})
					.collect::<Vec<_>>(),
			)
			.into_any(),
		Some(Err(e)) => div().class("text-red-600").child(format!("Error loading posts: {}", e)).into_any(),
	}
}
