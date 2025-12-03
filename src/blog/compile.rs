use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	process::Command,
	sync::OnceLock,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

static BLOG_POSTS: OnceLock<Vec<BlogPost>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct BlogPost {
	pub title: String,
	pub slug: String,
	pub created: DateTime<Utc>,
	pub html_path: PathBuf,
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

/// Compiles all .typ files in the blog directory to HTML.
/// Returns sorted list of blog posts (newest first).
pub fn compile_blog_posts(blog_dir: &Path, output_dir: &Path) -> Vec<BlogPost> {
	let mut posts = Vec::new();

	let typ_files: Vec<_> = match fs::read_dir(blog_dir) {
		Ok(entries) => entries.filter_map(|e| e.ok()).filter(|e| e.path().extension().map_or(false, |ext| ext == "typ")).collect(),
		Err(e) => {
			error!("Failed to read blog directory {:?}: {}", blog_dir, e);
			return posts;
		}
	};

	// Load existing metadata
	let meta_path = blog_dir.join("meta.json");
	let mut meta = BlogMeta::load(&meta_path);
	let mut meta_changed = false;

	// Collect current filenames for cleanup
	let current_files: std::collections::HashSet<String> = typ_files.iter().filter_map(|e| e.path().file_name().and_then(|n| n.to_str()).map(String::from)).collect();

	// Cleanup: remove entries for files that no longer exist
	let files_to_remove: Vec<String> = meta.files.keys().filter(|k| !current_files.contains(*k)).cloned().collect();
	for filename in files_to_remove {
		info!("Removing stale meta entry for deleted file: {}", filename);
		meta.files.remove(&filename);
		meta_changed = true;
	}

	for entry in typ_files {
		let path = entry.path();
		let filename = match path.file_name().and_then(|n| n.to_str()) {
			Some(name) => name.to_string(),
			None => continue,
		};

		// Get creation time from meta.json, or fall back to file metadata for new files
		let created: DateTime<Utc> = if let Some(&date) = meta.files.get(&filename) {
			date
		} else {
			// New file - get date from filesystem and record it
			let metadata = match fs::metadata(&path) {
				Ok(m) => m,
				Err(e) => {
					warn!("Failed to get metadata for {:?}: {}", path, e);
					continue;
				}
			};
			let date: DateTime<Utc> = metadata.created().or_else(|_| metadata.modified()).map(|t| t.into()).unwrap_or_else(|_| Utc::now());
			info!("Recording new blog file in meta.json: {} -> {}", filename, date);
			meta.files.insert(filename.clone(), date);
			meta_changed = true;
			date
		};

		// Read content to extract title
		let content = match fs::read_to_string(&path) {
			Ok(c) => c,
			Err(e) => {
				warn!("Failed to read {:?}: {}", path, e);
				continue;
			}
		};

		let title = extract_title(&content, &filename);
		let slug = to_slug(&filename);

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
					posts.push(BlogPost { title, slug, created, html_path });
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

/// Initialize blog posts at startup. Call this from main.rs
pub fn init_blog_posts(blog_dir: &Path, output_dir: &Path) {
	let posts = compile_blog_posts(blog_dir, output_dir);
	info!("Compiled {} blog posts", posts.len());
	let _ = BLOG_POSTS.set(posts);
}

/// Get the compiled blog posts
pub fn get_blog_posts() -> &'static Vec<BlogPost> {
	BLOG_POSTS.get_or_init(Vec::new)
}
