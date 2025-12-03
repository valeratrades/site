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
pub async fn get_posts(year: Option<i32>, month: Option<u32>, day: Option<u32>) -> Result<Vec<(String, String, String)>, ServerFnError> {
	use chrono::Datelike;
	// Returns Vec<(date_display, title, url)>
	let posts = compile::get_blog_posts();
	Ok(posts
		.iter()
		.filter(|p| year.map_or(true, |y| p.created.year() == y) && month.map_or(true, |m| p.created.month() == m) && day.map_or(true, |d| p.created.day() == d))
		.map(|p| {
			let date_display = p.created.format("%b %d, %Y").to_string();
			let url = format!("/blog/{}/{:02}/{:02}/{}.html", p.created.year(), p.created.month(), p.created.day(), p.slug);
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
	let posts = LocalResource::new(move || get_posts(None, None, None));

	move || match posts.get() {
		None => div().class("text-gray-500").child("Loading...").into_any(),
		Some(Ok(posts)) if posts.is_empty() => div().class("text-gray-500").child("No posts found.").into_any(),
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

/// Renders a blog listing page with optional date filters - used by axum handlers
#[cfg(feature = "ssr")]
pub fn render_blog_list_html(year: Option<i32>, month: Option<u32>, day: Option<u32>) -> String {
	use chrono::Datelike;

	let posts = compile::get_blog_posts();
	let filtered: Vec<_> = posts
		.iter()
		.filter(|p| year.map_or(true, |y| p.created.year() == y) && month.map_or(true, |m| p.created.month() == m) && day.map_or(true, |d| p.created.day() == d))
		.collect();

	let title = match (year, month, day) {
		(Some(y), Some(m), Some(d)) => format!("Blog - {}/{:02}/{:02}", y, m, d),
		(Some(y), Some(m), None) => format!("Blog - {}/{:02}", y, m),
		(Some(y), None, None) => format!("Blog - {}", y),
		_ => "Blog".to_string(),
	};

	let posts_html: String = if filtered.is_empty() {
		r#"<p style="color: #666;">No posts found.</p>"#.to_string()
	} else {
		filtered
			.iter()
			.map(|p| {
				let date_display = p.created.format("%b %d, %Y").to_string();
				let url = format!("/blog/{}/{:02}/{:02}/{}.html", p.created.year(), p.created.month(), p.created.day(), p.slug);
				format!(
					r#"<li style="padding: 0.25rem 0;"><span style="color: #666; font-variant-numeric: tabular-nums;">{}</span> <a href="{}" style="color: black;">{}</a><hr style="border-color: #d1d5db; margin-top: 0.25rem;"></li>"#,
					date_display, url, p.title
				)
			})
			.collect()
	};

	format!(
		r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>
<style>
html {{ display: flex; justify-content: center; }}
body {{ max-width: 42rem; width: 100%; padding: 1rem; font-family: system-ui, sans-serif; }}
a {{ text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
ul {{ list-style: none; padding: 0; margin: 0; }}
</style>
</head>
<body>
<h1>{}</h1>
<ul>{}</ul>
</body>
</html>"#,
		title, title, posts_html
	)
}
