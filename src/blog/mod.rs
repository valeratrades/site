use leptos::{ev, html::*, prelude::*};
use leptos_meta::{Title, TitleProps};
use leptos_routable::prelude::*;
use leptos_router::hooks::use_params_map;

#[cfg(feature = "ssr")]
pub mod compile;

/// Blog post data for the listing
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PostSummary {
	pub date_display: String,
	pub title: String,
	pub url: String,
	pub text_content: String,
}

#[derive(Routable)]
#[routes(transition = false)]
pub enum Routes {
	#[route(path = "/")]
	List,

	#[fallback]
	#[route(path = "/*any")]
	DateFilter,
}

#[component]
pub fn BlogView() -> impl IntoView {
	leptos_router::components::Outlet()
}

#[component]
fn ListView() -> impl IntoView {
	BlogListPage(BlogListPageProps { year: None, month: None, day: None })
}

/// Handles /blog/2025, /blog/2025/12, /blog/2025/12/03, and /blog/2025/12/03/post.html
#[component]
fn DateFilterView() -> impl IntoView {
	let params = use_params_map();
	let any = params.get().get("any").unwrap_or_default();
	let parts: Vec<&str> = any.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();

	// Check if this is a blog post (4 parts, last one ends with .html)
	if parts.len() == 4 && parts[3].ends_with(".html") {
		let slug = parts[3].trim_end_matches(".html").to_string();
		return BlogPostView(BlogPostViewProps { slug }).into_any();
	}

	// Otherwise it's a date filter
	let year = parts.first().and_then(|s| s.parse::<i32>().ok());
	let month = parts.get(1).and_then(|s| s.parse::<u32>().ok());
	let day = parts.get(2).and_then(|s| s.parse::<u32>().ok());

	BlogListPage(BlogListPageProps { year, month, day }).into_any()
}

#[server(GetBlogPost)]
pub async fn get_blog_post(slug: String) -> Result<Option<(String, String)>, ServerFnError> {
	let posts = compile::get_blog_posts();
	let post = posts.iter().find(|p| p.slug == slug);

	match post {
		Some(p) => {
			let content = std::fs::read_to_string(&p.html_path).map_err(|e| ServerFnError::new(e.to_string()))?;
			Ok(Some((p.title.clone(), content)))
		}
		None => Ok(None),
	}
}

#[component]
fn BlogPostView(slug: String) -> impl IntoView {
	let post = Resource::new(move || slug.clone(), |slug| get_blog_post(slug));

	move || {
		Suspend::new(async move {
			match post.await {
				Ok(None) => div()
					.class("max-w-2xl mx-auto px-4 py-8 text-center")
					.child((
						h1().class("text-2xl font-bold").child("Post Not Found"),
						a().attr("href", "/blog")
							.class("inline-block px-4 py-2 bg-green-600 text-white rounded mt-2")
							.child("Back to Blog"),
					))
					.into_any(),
				Ok(Some((title, content))) => div()
					.child((
						Title(TitleProps {
							formatter: None,
							text: Some(title.into()),
						}),
						div().class("blog-post-content max-w-2xl mx-auto px-4 py-8").inner_html(content),
					))
					.into_any(),
				Err(e) => div().class("max-w-2xl mx-auto px-4 py-8 text-red-600").child(format!("Error: {}", e)).into_any(),
			}
		})
	}
}

#[server(GetBlogPosts)]
pub async fn get_posts(year: Option<i32>, month: Option<u32>, day: Option<u32>) -> Result<Vec<PostSummary>, ServerFnError> {
	use chrono::Datelike;
	let posts = compile::get_blog_posts();
	Ok(posts
		.iter()
		.filter(|p| year.map_or(true, |y| p.created.year() == y) && month.map_or(true, |m| p.created.month() == m) && day.map_or(true, |d| p.created.day() == d))
		.map(|p| PostSummary {
			date_display: p.created.format("%b %d, %Y").to_string(),
			url: format!("/blog/{}/{:02}/{:02}/{}.html", p.created.year(), p.created.month(), p.created.day(), p.slug),
			title: p.title.clone(),
			text_content: p.text_content.clone(),
		})
		.collect())
}

#[component]
fn BlogListPage(year: Option<i32>, month: Option<u32>, day: Option<u32>) -> impl IntoView {
	let title = match (year, month, day) {
		(Some(y), Some(m), Some(d)) => format!("Blog - {}/{:02}/{:02}", y, m, d),
		(Some(y), Some(m), None) => format!("Blog - {}/{:02}", y, m),
		(Some(y), None, None) => format!("Blog - {}", y),
		_ => "Blog".to_string(),
	};

	let posts = Resource::new(move || (year, month, day), |(y, m, d)| get_posts(y, m, d));

	section().class("max-w-2xl mx-auto px-4 py-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some(title.clone().into()),
		}),
		h1().class("text-3xl font-bold mb-4").child(title),
		move || {
			Suspend::new(async move {
				match posts.await {
					Ok(posts) => BlogPostList(BlogPostListProps { posts }).into_any(),
					Err(e) => div().class("text-red-600").child(format!("Error: {}", e)).into_any(),
				}
			})
		},
	))
}

#[island]
fn BlogPostList(posts: Vec<PostSummary>) -> impl IntoView {
	let posts = StoredValue::new(posts);
	let (search_query, set_search_query) = signal(String::new());
	let (show_help, set_show_help) = signal(false);
	let (selected_index, set_selected_index) = signal::<Option<usize>>(None);
	let (visible_urls, set_visible_urls) = signal::<Vec<String>>(Vec::new());
	let search_ref = NodeRef::<leptos::html::Input>::new();

	// Keyboard shortcuts
	Effect::new(move |_| {
		use wasm_bindgen::{JsCast, closure::Closure};
		let window = web_sys::window().unwrap();
		let window_for_handler = window.clone();
		let document = window.document().unwrap();
		let search_input = search_ref.get();

		let handler = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
			let window = &window_for_handler;
			let active = document.active_element();
			let is_search_focused = active.as_ref().map_or(false, |el| el.tag_name() == "INPUT");

			if show_help.get_untracked() {
				if e.key() == "Escape" || e.key() == "?" {
					e.prevent_default();
					set_show_help.set(false);
				}
				return;
			}

			// Arrow navigation works both in search and globally
			let urls = visible_urls.get_untracked();
			let count = urls.len();
			match e.key().as_str() {
				"ArrowDown" => {
					e.prevent_default();
					if count > 0 {
						let current = selected_index.get_untracked();
						let next = match current {
							None => Some(0),
							Some(i) if i + 1 < count => Some(i + 1),
							Some(i) => Some(i), // Stay at last
						};
						set_selected_index.set(next);
					}
					return;
				}
				"ArrowUp" => {
					e.prevent_default();
					if count > 0 {
						let current = selected_index.get_untracked();
						let next = match current {
							None => Some(0),
							Some(0) => Some(0), // Stay at first
							Some(i) => Some(i - 1),
						};
						set_selected_index.set(next);
					}
					return;
				}
				"Enter" => {
					// Navigate to selected or single match
					let target_url = if count == 1 {
						urls.first().cloned()
					} else if let Some(idx) = selected_index.get_untracked() {
						urls.get(idx).cloned()
					} else {
						None
					};
					if let Some(url) = target_url {
						e.prevent_default();
						let _ = window.location().set_href(&url);
					}
					return;
				}
				_ => {}
			}

			if is_search_focused {
				if e.key() == "Escape" {
					e.prevent_default();
					set_search_query.set(String::new());
					set_selected_index.set(None);
					if let Some(input) = search_input.as_ref() {
						let _ = input.blur();
					}
				}
				return;
			}

			match e.key().as_str() {
				"s" | "S" | "/" => {
					e.prevent_default();
					if let Some(input) = search_input.as_ref() {
						let _ = input.focus();
					}
				}
				"?" => {
					e.prevent_default();
					set_show_help.set(true);
				}
				"Escape" => {
					set_selected_index.set(None);
				}
				_ => {}
			}
		});

		window.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref()).unwrap();
		handler.forget();
	});

	// Reset selection when search query changes
	Effect::new(move |_| {
		let _ = search_query.get();
		set_selected_index.set(None);
	});

	let search_input = input()
		.attr("type", "text")
		.attr("placeholder", "Type 'S' or '/' to search, '?' for help...")
		.attr("autocomplete", "off")
		.class("flex-1 px-3 py-2 border border-gray-400 rounded-l text-sm bg-gray-100 focus:bg-white focus:border-gray-500 focus:outline-none placeholder-gray-500")
		.node_ref(search_ref)
		.prop("value", move || search_query.get())
		.on(ev::input, move |e| {
			set_search_query.set(event_target_value(&e));
		});

	let help_button = button()
		.attr("type", "button")
		.attr("title", "Help (?)")
		.class("px-3 py-2 border border-l-0 border-gray-400 rounded-r bg-gray-200 hover:bg-gray-300 text-gray-600 hover:text-gray-800")
		.on(ev::click, move |_| set_show_help.set(true))
		.child("?");

	let search_box = div().class("flex mb-4").child((search_input, help_button));

	let help_modal = div()
		.class(move || {
			if show_help.get() {
				"fixed inset-0 bg-black/50 z-50 flex justify-center items-start pt-[10vh]"
			} else {
				"hidden"
			}
		})
		.on(ev::click, move |e| {
			// Close when clicking backdrop
			if event_target::<web_sys::HtmlElement>(&e).class_list().contains("fixed") {
				set_show_help.set(false);
			}
		})
		.child(div().class("bg-[#353535] text-gray-300 rounded-lg p-6 max-w-md w-[90%] shadow-xl").child((
			h2().class("text-white text-xl font-semibold mb-4").child("Keyboard Shortcuts"),
			div().class("space-y-2").child((
				help_row("?", "Show this help dialog"),
				help_row_multi(&["S", "/"], "Focus the search field"),
				help_row_multi(&["↑", "↓"], "Navigate through articles"),
				help_row("Enter", "Go to selected/single article"),
				help_row("Esc", "Clear search and close"),
			)),
		)));

	let posts_list = move || {
		let mut filtered_posts = posts.get_value();
		let query = search_query.get();

		if !query.is_empty() {
			// Score and sort posts
			let mut scored: Vec<_> = filtered_posts.iter().map(|p| (p.clone(), score_post(p, &query))).filter(|(_, s)| *s > 0.0).collect();
			scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
			filtered_posts = scored.into_iter().map(|(p, _)| p).collect();
		}

		if filtered_posts.is_empty() {
			set_visible_urls.set(Vec::new());
			return p().class("text-gray-500").child("No matching posts.").into_any();
		}

		// Update URLs for keyboard nav
		set_visible_urls.set(filtered_posts.iter().map(|p| p.url.clone()).collect());

		let selected = selected_index.get();

		ul().class("list-none p-0 m-0")
			.attr("id", "posts-list")
			.child(
				filtered_posts
					.into_iter()
					.enumerate()
					.map(|(idx, post)| {
						let is_selected = selected == Some(idx);
						let bg_class = if is_selected { "py-1 bg-blue-100 -mx-2 px-2 rounded" } else { "py-1" };
						li().class(bg_class).child((
							span().class("text-gray-500 tabular-nums").child(post.date_display),
							span().child(" "),
							a().attr("href", post.url)
								.class(if is_selected {
									"text-blue-700 font-medium hover:underline"
								} else {
									"text-black hover:underline"
								})
								.child(post.title),
							hr().class("border-gray-300 mt-1"),
						))
					})
					.collect::<Vec<_>>(),
			)
			.into_any()
	};

	div().child((search_box, posts_list, help_modal))
}

fn help_row(key: &str, desc: &str) -> impl IntoView {
	div().class("flex items-center gap-4").child((
		span()
			.class("inline-block bg-[#4a4a4a] border border-[#666] rounded px-2 py-0.5 font-mono text-sm min-w-[1.5rem] text-center")
			.child(key.to_string()),
		span().class("text-gray-400").child(desc.to_string()),
	))
}

fn help_row_multi(keys: &[&str], desc: &str) -> impl IntoView {
	div().class("flex items-center gap-4").child((
		div().class("flex items-center gap-1").child(
			keys.iter()
				.enumerate()
				.map(|(i, key)| {
					let sep = if i < keys.len() - 1 { " / " } else { "" };
					(
						span()
							.class("inline-block bg-[#4a4a4a] border border-[#666] rounded px-2 py-0.5 font-mono text-sm min-w-[1.5rem] text-center")
							.child(key.to_string()),
						span().class("text-gray-500").child(sep.to_string()),
					)
				})
				.collect::<Vec<_>>(),
		),
		span().class("text-gray-400").child(desc.to_string()),
	))
}

fn tokenize(s: &str) -> Vec<String> {
	s.to_lowercase().split_whitespace().filter(|t| !t.is_empty()).map(|s| s.to_string()).collect()
}

fn score_post(post: &PostSummary, query: &str) -> f64 {
	let query_tokens = tokenize(query);
	if query_tokens.is_empty() {
		return 0.0;
	}

	let title_tokens = tokenize(&post.title);
	let content_tokens = tokenize(&post.text_content);
	let title_lower = post.title.to_lowercase();
	let content_lower = post.text_content.to_lowercase();

	let mut total_score = 0.0;

	for qt in &query_tokens {
		let mut term_score = 0.0;

		// Title matches (10x weight)
		for tt in &title_tokens {
			if tt == qt {
				term_score += 100.0;
			} else if tt.starts_with(qt) {
				term_score += 50.0;
			} else if tt.contains(qt) {
				term_score += 20.0;
			}
		}

		// Content matches (1x weight)
		for ct in &content_tokens {
			if ct == qt {
				term_score += 10.0;
			} else if ct.starts_with(qt) {
				term_score += 5.0;
			} else if ct.contains(qt) {
				term_score += 2.0;
			}
		}

		// Phrase matching bonus
		if title_lower.contains(qt) {
			term_score += 10.0;
		}
		if content_lower.contains(qt) {
			term_score += 1.0;
		}

		total_score += term_score;
	}

	total_score / query_tokens.len() as f64
}
