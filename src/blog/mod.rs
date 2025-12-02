use leptos::{html::*, prelude::*};
use leptos_meta::{Title, TitleProps};
use leptos_routable::prelude::*;
use leptos_router::components::{A, AProps};

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
		A(AProps {
			href: "/blog".to_string(),
			children: Box::new(|| view! { "Back to Blog" }.into_any()),
			target: None,
			exact: false,
			strict_trailing_slash: false,
			scroll: true,
		})
		.attr("class", "inline-block px-4 py-2 bg-green-600 text-white rounded mt-2"),
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

#[component]
fn HomeView() -> impl IntoView {
	section().class("max-w-2xl mx-auto px-4 py-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Blog".into()),
		}),
		// Navigation header like matklad
		nav().class("mb-8 flex gap-4 text-gray-600").child((
			A(AProps {
				href: "/".to_string(),
				children: Box::new(|| span().class("hover:text-gray-900").child("home").into_any()),
				target: None,
				exact: false,
				strict_trailing_slash: false,
				scroll: true,
			}),
			A(AProps {
				href: "/contacts".to_string(),
				children: Box::new(|| span().class("hover:text-gray-900").child("contacts").into_any()),
				target: None,
				exact: false,
				strict_trailing_slash: false,
				scroll: true,
			}),
		)),
		// Blog post list
		BlogPostList(),
	))
}

#[island]
fn BlogPostList() -> impl IntoView {
	let posts = LocalResource::new(get_posts);

	move || match posts.get() {
		None => div().child("Loading...").into_any(),
		Some(Ok(posts)) => ul()
			.class("space-y-0")
			.child(
				posts
					.into_iter()
					.map(|(date, title, url)| {
						li().class("py-2 border-b border-gray-200").child((
							span().class("text-gray-500 mr-4 font-mono text-sm").child(date),
							a().attr("href", url).class("text-blue-600 hover:text-blue-800 hover:underline").child(title),
						))
					})
					.collect::<Vec<_>>(),
			)
			.into_any(),
		Some(Err(e)) => div().class("text-red-600").child(format!("Error loading posts: {}", e)).into_any(),
	}
}
