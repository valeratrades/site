use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	process::Command,
	sync::RwLock,
};

use chrono::{DateTime, Utc};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

static BLOG_POSTS: RwLock<Vec<BlogPost>> = RwLock::new(Vec::new());

#[derive(Clone, Debug)]
pub struct BlogPost {
	pub title: String,
	pub slug: String,
	pub created: DateTime<Utc>,
	pub html_path: PathBuf,
	pub text_content: String,
}

/// Metadata for blog posts, stored in meta.json
#[derive(Default, Deserialize, Serialize)]
struct BlogMeta {
	/// Map from filename (e.g., "my_post.typ") to creation datetime
	files: HashMap<String, DateTime<Utc>>,
}

impl BlogMeta {
	fn load(meta_path: &Path) -> Self {
		match fs::read_to_string(meta_path) {
			Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
			Err(_) => Self::default(),
		}
	}

	fn save(&self, meta_path: &Path) -> Result<(), std::io::Error> {
		let json = serde_json::to_string_pretty(self)?;
		fs::write(meta_path, json)
	}
}

/// Extracts title from typst file content.
/// Looks for first heading (= Title) or uses filename.
fn extract_title(content: &str, filename: &str) -> String {
	for line in content.lines() {
		let trimmed = line.trim();
		if trimmed.starts_with('=') && !trimmed.starts_with("==") {
			// Single = is h1 in typst
			return trimmed.trim_start_matches('=').trim().to_string();
		}
	}
	// Fallback to filename
	filename
		.trim_end_matches(".typ")
		.replace('_', " ")
		.split_whitespace()
		.map(|word| {
			let mut chars = word.chars();
			match chars.next() {
				None => String::new(),
				Some(c) => c.to_uppercase().chain(chars).collect(),
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

/// Converts filename to URL slug
fn to_slug(filename: &str) -> String {
	filename.trim_end_matches(".typ").to_lowercase().replace(' ', "-")
}

/// Extracts plain text content from an HTML file for search indexing
fn extract_text_from_html(html_path: &Path) -> String {
	let html = match fs::read_to_string(html_path) {
		Ok(content) => content,
		Err(e) => {
			warn!("Failed to read HTML for text extraction {:?}: {}", html_path, e);
			return String::new();
		}
	};

	// Simple HTML tag stripping - extract text content
	let mut text = String::new();
	let mut in_tag = false;
	let mut in_script = false;
	let mut in_style = false;

	let html_lower = html.to_lowercase();
	let chars: Vec<char> = html.chars().collect();
	let chars_lower: Vec<char> = html_lower.chars().collect();

	let mut i = 0;
	while i < chars.len() {
		if chars[i] == '<' {
			// Check for script/style tags
			let remaining: String = chars_lower[i..].iter().collect();
			if remaining.starts_with("<script") {
				in_script = true;
			} else if remaining.starts_with("</script") {
				in_script = false;
			} else if remaining.starts_with("<style") {
				in_style = true;
			} else if remaining.starts_with("</style") {
				in_style = false;
			}
			in_tag = true;
		} else if chars[i] == '>' {
			in_tag = false;
		} else if !in_tag && !in_script && !in_style {
			text.push(chars[i]);
		}
		i += 1;
	}

	// Decode common HTML entities and normalize whitespace
	text.replace("&nbsp;", " ")
		.replace("&amp;", "&")
		.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&#39;", "'")
		.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ")
}

/// Represents a discovered blog source: either a standalone .typ file or a directory with mod.typ
struct BlogSource {
	/// The .typ file to compile
	typ_path: PathBuf,
	/// The canonical name for this post (filename without .typ, or directory name)
	name: String,
}

/// Discovers blog sources: both `article.typ` files and `article/mod.typ` directories
fn discover_blog_sources(blog_dir: &Path) -> Vec<BlogSource> {
	let mut sources = Vec::new();

	let entries = match fs::read_dir(blog_dir) {
		Ok(e) => e,
		Err(e) => {
			error!("Failed to read blog directory {:?}: {}", blog_dir, e);
			return sources;
		}
	};

	for entry in entries.filter_map(|e| e.ok()) {
		let path = entry.path();

		if path.is_file() && path.extension().map_or(false, |ext| ext == "typ") {
			// Standalone .typ file (e.g., my_article.typ)
			if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
				sources.push(BlogSource {
					typ_path: path.clone(),
					name: name.to_string(),
				});
			}
		} else if path.is_dir() {
			// Check for mod.typ inside directory (e.g., my_article/mod.typ)
			let mod_path = path.join("mod.typ");
			if mod_path.exists() {
				if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
					sources.push(BlogSource {
						typ_path: mod_path,
						name: name.to_string(),
					});
				}
			}
		}
	}

	sources
}

/// Compiles all .typ files in the blog directory to HTML.
/// Returns sorted list of blog posts (newest first).
pub fn compile_blog_posts(blog_dir: &Path, output_dir: &Path) -> Vec<BlogPost> {
	let mut posts = Vec::new();

	let blog_sources = discover_blog_sources(blog_dir);

	// Load existing metadata
	let meta_path = blog_dir.join("meta.json");
	let mut meta = BlogMeta::load(&meta_path);
	let mut meta_changed = false;

	// Collect current names for cleanup (uses canonical name, not filename)
	let current_names: std::collections::HashSet<String> = blog_sources.iter().map(|s| s.name.clone()).collect();

	// Cleanup: remove entries for files that no longer exist
	let names_to_remove: Vec<String> = meta.files.keys().filter(|k| !current_names.contains(*k)).cloned().collect();
	for name in names_to_remove {
		info!("Removing stale meta entry for deleted post: {}", name);
		meta.files.remove(&name);
		meta_changed = true;
	}

	for source in blog_sources {
		let path = &source.typ_path;
		let name = &source.name;

		// Get creation time from meta.json, or fall back to file metadata for new files
		let created: DateTime<Utc> = if let Some(&date) = meta.files.get(name) {
			date
		} else {
			// New file - get date from filesystem and record it
			let metadata = match fs::metadata(path) {
				Ok(m) => m,
				Err(e) => {
					warn!("Failed to get metadata for {:?}: {}", path, e);
					continue;
				}
			};
			let date: DateTime<Utc> = metadata.created().or_else(|_| metadata.modified()).map(|t| t.into()).unwrap_or_else(|_| Utc::now());
			info!("Recording new blog post in meta.json: {} -> {}", name, date);
			meta.files.insert(name.clone(), date);
			meta_changed = true;
			date
		};

		// Read content to extract title
		let content = match fs::read_to_string(path) {
			Ok(c) => c,
			Err(e) => {
				warn!("Failed to read {:?}: {}", path, e);
				continue;
			}
		};

		let title = extract_title(&content, name);
		let slug = to_slug(name);

		// Create output directory structure: output_dir/YYYY/MM/DD/
		let year = created.format("%Y").to_string();
		let month = created.format("%m").to_string();
		let day = created.format("%d").to_string();

		let post_output_dir = output_dir.join(&year).join(&month).join(&day);
		if let Err(e) = fs::create_dir_all(&post_output_dir) {
			error!("Failed to create directory {:?}: {}", post_output_dir, e);
			continue;
		}

		let html_filename = format!("{}.html", slug);
		let html_path = post_output_dir.join(&html_filename);

		// Compile typst to HTML
		let output = Command::new("typst").args(["compile", "--features", "html"]).arg(&path).arg(&html_path).output();

		match output {
			Ok(result) =>
				if result.status.success() {
					info!("Compiled {:?} -> {:?}", path, html_path);
					// Extract text content from HTML for search indexing
					let text_content = extract_text_from_html(&html_path);
					posts.push(BlogPost {
						title,
						slug,
						created,
						html_path,
						text_content,
					});
				} else {
					let stderr = String::from_utf8_lossy(&result.stderr);
					error!("typst compile failed for {:?}: {}", path, stderr);
				},
			Err(e) => {
				error!("Failed to run typst for {:?}: {}", path, e);
			}
		}
	}

	// Save metadata if changed
	if meta_changed {
		if let Err(e) = meta.save(&meta_path) {
			error!("Failed to save blog meta.json: {}", e);
		}
	}

	// Sort by creation time, newest first
	posts.sort_by(|a, b| b.created.cmp(&a.created));
	posts
}

/// Initialize blog posts at startup and start file watcher. Call this from main.rs.
/// Returns the watcher handle which must be kept alive to continue watching.
pub fn init_blog_posts(blog_dir: &Path, output_dir: &Path) -> RecommendedWatcher {
	// Initial compilation
	let posts = compile_blog_posts(blog_dir, output_dir);
	info!("Compiled {} blog posts", posts.len());
	*BLOG_POSTS.write().unwrap() = posts;

	// Set up file watcher
	let blog_dir_owned = blog_dir.to_path_buf();
	let output_dir_owned = output_dir.to_path_buf();

	let mut watcher = RecommendedWatcher::new(
		move |res: Result<notify::Event, notify::Error>| match res {
			Ok(event) => {
				// Only recompile on relevant events
				let dominated_by_typ = event.paths.iter().any(|p| p.extension().map_or(false, |ext| ext == "typ"));
				let dominated_by_json = event.paths.iter().any(|p| p.file_name().map_or(false, |n| n == "meta.json"));
				let dominated_by_html = event.paths.iter().all(|p| p.extension().map_or(false, |ext| ext == "html"));

				// Skip if all paths are HTML files (our own output)
				if dominated_by_html {
					return;
				}

				// Only recompile for typ file changes, or meta.json changes, or file creation/deletion
				let is_modification = matches!(event.kind, notify::EventKind::Modify(ModifyKind::Data(_)));
				let is_structural = matches!(event.kind, notify::EventKind::Create(_) | notify::EventKind::Remove(_));

				if (is_modification && (dominated_by_typ || dominated_by_json)) || is_structural {
					info!("Blog directory changed ({:?}), recompiling...", event.kind);
					let posts = compile_blog_posts(&blog_dir_owned, &output_dir_owned);
					info!("Recompiled {} blog posts", posts.len());
					*BLOG_POSTS.write().unwrap() = posts;
				}
			}
			Err(e) => error!("File watch error: {:?}", e),
		},
		Config::default(),
	)
	.expect("Failed to create file watcher");

	watcher.watch(blog_dir, RecursiveMode::Recursive).expect("Failed to watch blog directory");
	info!("Watching {:?} for changes", blog_dir);

	watcher
}

/// Get the compiled blog posts
pub fn get_blog_posts() -> Vec<BlogPost> {
	BLOG_POSTS.read().unwrap().clone()
}

/// Get a blog post title by slug
pub fn get_post_title(slug: &str) -> Option<String> {
	get_blog_posts().iter().find(|p| p.slug == slug).map(|p| p.title.clone())
}
