#[cfg(feature = "ssr")]
mod data;
use leptos::{html::*, prelude::*};

use crate::conf::Settings;
#[cfg(feature = "ssr")]
use crate::utils::Mock;

#[component]
pub fn CftcReportView() -> impl IntoView {
	let report_resource = Resource::new(move || (), |_| async move { try_build().await.expect("dbg") });
	div().child(Suspense(SuspenseProps {
		fallback: { || pre().child("Loading CFTC Report...") }.into(),
		#[rustfmt::skip]
		children: ToChildren::to_children(move || IntoRender::into_render(move || {
			report_resource.get().map(|ms| pre().child(ms.short))
		})),
	}))
}

#[server]
async fn try_build() -> Result<CftcReportRendered, ServerFnError> {
	crate::try_load_mock!(data::CftcReport; .into());

	let report = data::fetch_cftc_positions().await.expect("TODO: proper error handling");
	report.persist()?;
	Ok(report.into())
}
#[cfg(feature = "ssr")]
impl Mock for data::CftcReport {}
#[derive(Clone, Debug, Default, derive_new::new, serde::Deserialize, serde::Serialize)]
pub struct CftcReportRendered {
	short: String,
	markdown: String,
}
#[cfg(feature = "ssr")]
impl From<data::CftcReport> for CftcReportRendered {
	fn from(report: data::CftcReport) -> Self {
		Self::new(report.to_string(), report.to_markdown_table())
	}
}
