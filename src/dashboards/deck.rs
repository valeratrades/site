//! The dashboard rendered as a packed-grid dock: the five sub-dashboards become draggable/resizable
//! panels. `Alt+S` caches the live arrangement in this browser (dockviewers does that itself);
//! `Alt+Shift+S` publishes it, admin-only, as the default every other visitor lands on. This island
//! is the whole dashboard now — its child views are plain components that hydrate within it.

#[cfg(feature = "ssr")]
use std::path::PathBuf;
use std::{rc::Rc, sync::Arc};

use dockviewers::leptos::{Config, DockPanel, Group, MinSize, PackedApi, PackedArea, PanelId, Saved, Step};
use leptos::prelude::*;

use super::{cme, fng, lsr, market_structure, vol};

/// Every panel this deck hosts. A layout that doesn't cover all of these is treated as unusable and
/// replaced by the seed.
const PANEL_IDS: [&str; 5] = ["market_structure", "lsr", "cme", "vol", "fng"];
#[island]
pub fn DashboardDeck() -> impl IntoView {
	let panels = RwSignal::new(vec![
		DockPanel {
			id: PanelId("market_structure".into()),
			title: "Market Structure".into(),
			content: Arc::new(|| market_structure::MarketStructureView().into_any()),
		},
		DockPanel {
			id: PanelId("lsr".into()),
			title: "LSR".into(),
			content: Arc::new(|| lsr::LsrView().into_any()),
		},
		DockPanel {
			id: PanelId("cme".into()),
			title: "CFTC".into(),
			content: Arc::new(|| cme::CftcReportView().into_any()),
		},
		DockPanel {
			id: PanelId("vol".into()),
			title: "Vol".into(),
			content: Arc::new(|| vol::VolView().into_any()),
		},
		DockPanel {
			id: PanelId("fng".into()),
			title: "Fear & Greed".into(),
			content: Arc::new(|| fng::FngView().into_any()),
		},
	]);

	// Fires once per band entry, on the client, after dockviewers has already resolved its own
	// localStorage cache — so this only has to cover what the cache didn't: the published default,
	// then the built-in seed.
	let on_band = Arc::new(move |api: PackedApi| {
		if api.restored() {
			if !hosts_every_panel(&api) {
				leptos::logging::error!("cached layout is missing panels, using seed");
				seed(&api);
			}
			return;
		}
		let band = api.band();
		leptos::task::spawn_local(async move {
			let loaded = load_layout(band.to_string()).await;
			// A resize can cross into another band while this is in flight; that crossing ran its own
			// `on_band`, and applying a stale band's layout over it would fight the newer one.
			if api.band() != band {
				return;
			}
			match loaded {
				Ok(Some(json)) => {
					// A parseable layout can still be unusable — empty, or missing panels (published
					// before a panel existed, or a truncated write) — and renders as a black empty dock.
					if !(api.load(&json).is_ok() && hosts_every_panel(&api)) {
						leptos::logging::error!("published layout unusable (corrupt or missing panels), using seed");
						seed(&api);
					}
				}
				Ok(None) => seed(&api),
				Err(e) => {
					leptos::logging::error!("load_layout failed, using seed: {e}");
					seed(&api);
				}
			}
		});
	}) as Arc<dyn Fn(PackedApi) + Send + Sync>;

	// A real height so the dock's first measure lands; the app nav sits above it. Defaults already
	// ship a dark theme, so only the accent is nudged to the site's green.
	// ponytail: 3.5rem tracks the nav's `py-2` + h-8 avatar; retune if the nav height changes.
	// PackedState holds `Rc`s → the dock's `RwSignal::new_local` wraps a `!Send` value in a
	// `SendWrapper`, which panics if built during SSR (the streamed render disposes its owner on a
	// different worker thread). Gate the dock behind a client-only `mounted` flag: server and client
	// both first render the empty host (hydration matches), then this effect — which never runs on the
	// server — flips it and the dock is built on the wasm thread.
	let mounted = RwSignal::new(false);
	Effect::new(move |_| mounted.set(true));

	// Save feedback: the save hook sets this, the overlay below shows it, then it self-clears.
	let toast = RwSignal::new(None::<String>);

	view! {
		<div
			class="dv-host"
			style="position:relative; height:calc(100vh - 3.5rem); --dv-accent:#22c55e;"
		>
			<Show when=move || mounted.get() fallback=|| ()>
				<PackedArea panels=panels config=dock_config(toast) on_band=on_band.clone() />
			</Show>
			{move || {
				toast
					.get()
					.map(|msg| {
						view! {
							<div style="pointer-events:none;position:fixed;bottom:1.5rem;left:50%;transform:translateX(-50%);z-index:50;border-radius:0.375rem;border:1px solid #22c55e55;background:rgba(0,0,0,0.85);padding:0.5rem 1rem;font:12px ui-monospace,monospace;letter-spacing:0.05em;color:#22c55e;box-shadow:0 4px 12px rgba(0,0,0,0.4)">
								{msg}
							</div>
						}
					})
			}}
		</div>
	}
}

/// Whether the live layout actually hosts all of [`PANEL_IDS`] — the difference between a layout and
/// a black rectangle.
fn hosts_every_panel(api: &PackedApi) -> bool {
	let live: std::collections::HashSet<String> = api.tab_ids().into_iter().map(|p| p.0).collect();
	PANEL_IDS.iter().all(|id| live.contains(*id))
}

/// Built-in first-run arrangement: each panel its own group, packed left→right. Sizes are in grid
/// steps (~64 cols × 36 rows fill the container); mins are `Rem` so a panel can't shrink below its
/// content's natural extent — the text panels floor at roughly their one/few readable lines, while
/// the chart keeps an elastic-but-sane range.
fn seed(api: &PackedApi) {
	api.reset();
	let specs: [(&str, u32, u32, MinSize); 5] = [
		// floored at the current live session size — these two never work any smaller
		("market_structure", 29, 16, MinSize::Steps { w: Step(29), h: Step(16) }),
		("lsr", 22, 16, MinSize::Steps { w: Step(11), h: Step(9) }),
		("cme", 20, 12, MinSize::Rem { w: 24.0, h: 8.0 }),
		("vol", 14, 4, MinSize::Rem { w: 16.0, h: 3.0 }),
		("fng", 16, 4, MinSize::Rem { w: 20.0, h: 3.0 }),
	];
	debug_assert!(
		specs.len() == PANEL_IDS.len() && PANEL_IDS.iter().all(|id| specs.iter().any(|(s, ..)| s == id)),
		"seed must cover exactly PANEL_IDS"
	);
	for (id, w, h, min) in specs {
		let group = Group::new(api.mint_group_id(), PanelId(id.into()));
		api.place(group, w, h, min);
	}
}

/// `Alt+S` is dockviewers' own per-band localStorage cache — nothing to do but say so. `Alt+Shift+S`
/// is this host's addition: publish the arrangement as the default fresh visitors get, which the
/// server rejects for non-admins. Built fresh per render so the `!Send` `Rc` hook is born on the
/// client, not captured by the island view.
fn dock_config(toast: RwSignal<Option<String>>) -> Config {
	Config {
		storage_key: Some("site-dashboard".into()),
		on_save: Some(Rc::new(move |saved| match saved {
			Saved::Cached { band } => show_toast(toast, format!("Layout cached ({band})")),
			Saved::Published { band, json } => leptos::task::spawn_local(async move {
				let msg = match save_layout(band.to_string(), json).await {
					Ok(()) => format!("Layout published ({band})"),
					Err(e) => {
						leptos::logging::error!("save_layout failed: {e}");
						"Publish failed".into()
					}
				};
				show_toast(toast, msg);
			}),
		})),
		..Default::default()
	}
}

fn show_toast(toast: RwSignal<Option<String>>, msg: String) {
	toast.set(Some(msg));
	#[cfg(target_arch = "wasm32")]
	leptos::task::spawn_local(async move {
		gloo_timers::future::TimeoutFuture::new(2500).await;
		toast.set(None);
	});
}

#[cfg(feature = "ssr")]
fn layout_path(key: &str) -> PathBuf {
	v_utils::xdg_cache_dir!("dashboards").join(format!("layout-{key}.json"))
}

/// The site-wide default for one band. Admin-only: this is what every visitor who hasn't cached
/// their own arrangement lands on.
#[server]
async fn save_layout(key: String, json: String) -> Result<(), ServerFnError> {
	crate::admin::require_admin().await?;
	std::fs::write(layout_path(&key), json).map_err(|e| ServerFnError::new(format!("write layout: {e}")))?;
	Ok(())
}

#[server]
async fn load_layout(key: String) -> Result<Option<String>, ServerFnError> {
	match std::fs::read_to_string(layout_path(&key)) {
		Ok(s) => Ok(Some(s)),
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
		Err(e) => Err(ServerFnError::new(format!("read layout: {e}"))),
	}
}
