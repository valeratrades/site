// check that the importmap script in `shell` (src/app.rs) SSRs with its body intact —
// the view-macro static path silently emitted `inner_html` as a literal attribute
fn main() {
	use leptos::prelude::{CustomAttribute as _, InnerHtmlAttribute as _};
	let html = leptos::prelude::RenderHtml::to_html(
		leptos::html::script()
			.attr("type", "importmap")
			.inner_html(r#"{"imports":{"lightweight-charts":"https://cdn.jsdelivr.net/npm/lightweight-charts@5/dist/lightweight-charts.standalone.production.mjs"}}"#),
	);
	println!("{html}");
	assert!(
		html.contains(r#"<script type="importmap">{"imports":{"lightweight-charts""#),
		"importmap body missing from SSR output"
	);
	println!("OK");
}
