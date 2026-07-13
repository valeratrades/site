#[cfg(feature = "ssr")]
mod data;
use leptos::{html::*, prelude::*};

use super::{LoadingIndicator, LoadingIndicatorProps};

#[component]
pub fn CftcReportView() -> impl IntoView {
	let trigger = RwSignal::new(());
	let report_resource = Resource::new(move || trigger.get(), |_| async move { try_build().await });

	// Set up retry interval - retry every 1 minute on error
	#[cfg(not(feature = "ssr"))]
	{
		Effect::new(move || {
			if let Some(Err(_)) = report_resource.get() {
				set_timeout(
					move || {
						trigger.update(|_| ());
					},
					std::time::Duration::from_secs(60),
				);
			}
		});
	}

	div().class("panel-center").child(Suspense(SuspenseProps {
		fallback: { || LoadingIndicator(LoadingIndicatorProps { label: "CFTC Report".into() }) }.into(),
		#[rustfmt::skip]
		children: ToChildren::to_children(move || IntoRender::into_render(move || match report_resource.get() {
			Some(Ok(report_data)) => (pre().child(report_data.short),).into_any(),
			Some(Err(e)) => (pre().child(format!("Error loading CFTC Report: {e} (retrying...)")),).into_any(),
			None => (LoadingIndicator(LoadingIndicatorProps { label: "CFTC Report".into() }),).into_any(),
		})),
	}))
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, derive_new::new)]
pub struct CftcReportRendered {
	short: String,
	markdown: String,
}
#[server]
async fn try_build() -> Result<CftcReportRendered, ServerFnError> {
	let report = super::_core::load::<data::CftcReport>()
		.await
		.map_err(|e| {
			tracing::error!("Failed to load CFTC positions: {e:?}");
			ServerFnError::new(format!("Failed to load CFTC positions: {e}"))
		})?
		.data;
	Ok(report.into())
}
#[cfg(feature = "ssr")]
impl super::_core::SourceData for data::CftcReport {
	fn decay_horizon() -> v_utils::trades::Timeframe {
		"6h".into() // report publishes weekly; a few polls/day catches the Friday release
	}

	async fn fetch() -> color_eyre::eyre::Result<Self> {
		data::fetch_cftc_positions().await
	}
}
#[cfg(feature = "ssr")]
impl From<data::CftcReport> for CftcReportRendered {
	fn from(report: data::CftcReport) -> Self {
		Self::new(report.to_string(), report.to_markdown_table())
	}
}
