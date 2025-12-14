use leptos::{html::*, prelude::*};
use leptos_routable::prelude::*;

#[cfg(feature = "ssr")]
pub mod compile;

#[derive(Routable)]
#[routes(transition = false)]
pub enum Routes {
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
				// Escape content for safe embedding in HTML attribute
				let escaped_content = p.text_content
					.replace('&', "&amp;")
					.replace('"', "&quot;")
					.replace('<', "&lt;")
					.replace('>', "&gt;");
				format!(
					r#"<li style="padding: 0.25rem 0;" data-content="{}"><span style="color: #666; font-variant-numeric: tabular-nums;">{}</span> <a href="{}" style="color: black;">{}</a><hr style="border-color: #d1d5db; margin-top: 0.25rem;"></li>"#,
					escaped_content, date_display, url, p.title
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
.search-box {{ width: 100%; padding: 0.5rem; margin-bottom: 1rem; border: 1px solid #d1d5db; border-radius: 0.25rem; font-size: 1rem; }}
.search-box:focus {{ outline: none; border-color: #9ca3af; }}
.no-results {{ color: #666; display: none; }}
</style>
</head>
<body>
<h1>{}</h1>
<input type="text" class="search-box" id="search" placeholder="Search posts..." autocomplete="off">
<p class="no-results" id="no-results">No matching posts.</p>
<ul id="posts-list">{}</ul>
<script>
(function() {{
  const search = document.getElementById('search');
  const list = document.getElementById('posts-list');
  const noResults = document.getElementById('no-results');
  const items = Array.from(list.querySelectorAll('li'));

  // Store original order and extract searchable text
  const posts = items.map((li, idx) => {{
    const link = li.querySelector('a');
    const title = link ? link.textContent.toLowerCase() : '';
    const content = (li.dataset.content || '').toLowerCase();
    return {{ el: li, title, content, origIdx: idx }};
  }});

  function tokenize(str) {{
    return str.toLowerCase().split(/\s+/).filter(t => t.length > 0);
  }}

  // Weighted scoring: title matches worth 10x content matches
  function score(post, queryTokens) {{
    if (queryTokens.length === 0) return 0;

    const titleTokens = tokenize(post.title);
    const contentTokens = tokenize(post.content);
    let totalScore = 0;

    for (const qt of queryTokens) {{
      let termScore = 0;

      // Title matches (10x weight)
      for (const tt of titleTokens) {{
        if (tt === qt) {{
          termScore += 100;  // Exact match in title
        }} else if (tt.startsWith(qt)) {{
          termScore += 50;   // Prefix match in title
        }} else if (tt.includes(qt)) {{
          termScore += 20;   // Substring match in title
        }}
      }}

      // Content matches (1x weight)
      for (const ct of contentTokens) {{
        if (ct === qt) {{
          termScore += 10;   // Exact match in content
        }} else if (ct.startsWith(qt)) {{
          termScore += 5;    // Prefix match in content
        }} else if (ct.includes(qt)) {{
          termScore += 2;    // Substring match in content
        }}
      }}

      // Phrase matching bonus
      if (post.title.includes(qt)) {{
        termScore += 10;
      }}
      if (post.content.includes(qt)) {{
        termScore += 1;
      }}

      totalScore += termScore;
    }}

    // Normalize by query length to not overly favor longer queries
    return totalScore / queryTokens.length;
  }}

  function doSearch() {{
    const query = search.value.trim();
    const queryTokens = tokenize(query);

    if (query === '') {{
      // Restore original order
      posts.sort((a, b) => a.origIdx - b.origIdx);
      posts.forEach(p => {{
        p.el.style.display = '';
        list.appendChild(p.el);
      }});
      noResults.style.display = 'none';
      return;
    }}

    // Score and sort
    const scored = posts.map(p => ({{ ...p, score: score(p, queryTokens) }}));
    scored.sort((a, b) => b.score - a.score);

    let visibleCount = 0;
    scored.forEach(p => {{
      if (p.score > 0) {{
        p.el.style.display = '';
        visibleCount++;
      }} else {{
        p.el.style.display = 'none';
      }}
      list.appendChild(p.el);
    }});

    noResults.style.display = visibleCount === 0 ? 'block' : 'none';
  }}

  search.addEventListener('input', doSearch);

  // Press 'S' to focus search bar
  document.addEventListener('keydown', (e) => {{
    if (e.key === 's' || e.key === 'S') {{
      if (document.activeElement !== search) {{
        e.preventDefault();
        search.focus();
      }}
    }}
  }});
}})();
</script>
</body>
</html>"#,
		title, title, posts_html
	)
}
