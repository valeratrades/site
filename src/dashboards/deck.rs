//! The dashboard rendered as a packed-grid dock: the five sub-dashboards become draggable/resizable
//! panels. `s` saves the live arrangement; three saved layouts are kept, keyed by device band, so
//! each screen class opens onto a sensible seed. This island is the whole dashboard now — its child
//! views are plain components that hydrate within it.

use std::{cell::Cell, path::PathBuf, rc::Rc, sync::Arc};

use dockviewers::leptos::{Breakpoint, Config, DockPanel, Group, Keybind, MinSize, PackedApi, PackedArea, PackedState, PanelId};
use leptos::prelude::*;

use super::{cme, fng, lsr, market_structure, vol};

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

	// All client wiring lives here: `on_ready` fires once on the client with a `Copy` (`!Send`)
	// `PackedApi`, so the reactive handles it spins up never cross the `Send + Sync` bound the prop
	// demands. The `Rc<Cell>` remembers the band group last loaded so the effect — which re-fires on
	// every layout edit (any `api.breakpoint()` read subscribes to the whole state) — only reloads
	// when a resize actually crosses a band boundary.
	let on_ready = Arc::new(move |api: PackedApi| {
		let loaded: Rc<Cell<Option<&'static str>>> = Rc::new(Cell::new(None));
		Effect::new(move |_| {
			let key = seed_key(api.breakpoint());
			if loaded.get() == Some(key) {
				return;
			}
			loaded.set(Some(key));
			leptos::task::spawn_local(async move {
				match load_layout(key.to_string()).await {
					Ok(Some(json)) =>
						if let Err(e) = api.load(&json) {
							leptos::logging::error!("saved layout corrupt, using seed: {e:?}");
							seed(&api);
						},
					Ok(None) => seed(&api),
					Err(e) => {
						leptos::logging::error!("load_layout failed, using seed: {e}");
						seed(&api);
					}
				}
			});
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

	view! {
		<div
			class="dv-host"
			style="position:relative; height:calc(100vh - 3.5rem); --dv-accent:#22c55e;"
		>
			<Show when=move || mounted.get() fallback=|| ()>
				<PackedArea panels=panels config=keybinds() on_ready=on_ready.clone() />
			</Show>
		</div>
	}
}
/// Coarse device band → saved-layout key. Xl/Lg share `xl`, Sm/Xs share `sm`.
fn seed_key(bp: Breakpoint) -> &'static str {
	use Breakpoint::*;
	match bp {
		Xl | Lg => "xl",
		Md => "md",
		Sm | Xs => "sm",
	}
}

/// Built-in first-run arrangement: each panel its own group, packed left→right. Sizes are in grid
/// steps (~64 cols × 36 rows fill the container); mins are `Rem` so a panel can't shrink below its
/// content's natural extent — the text panels floor at roughly their one/few readable lines, while
/// the chart keeps an elastic-but-sane range.
fn seed(api: &PackedApi) {
	api.reset();
	let specs: [(&str, u32, u32, MinSize); 5] = [
		("market_structure", 26, 16, MinSize::Rem { w: 22.0, h: 14.0 }),
		("lsr", 22, 16, MinSize::Rem { w: 22.0, h: 12.0 }),
		("cme", 20, 12, MinSize::Rem { w: 24.0, h: 8.0 }),
		("vol", 14, 4, MinSize::Rem { w: 16.0, h: 3.0 }),
		("fng", 16, 4, MinSize::Rem { w: 20.0, h: 3.0 }),
	];
	for (id, w, h, min) in specs {
		let group = Group::new(api.mint_group_id(), PanelId(id.into()));
		api.place(group, w, h, min);
	}
}

/// `s` persists the live arrangement under its band key, via dockviewers' own keydown path (the same
/// one that drives `u`/`f`/`?`), so it inherits its editable-field guard and hydration timing. Built
/// fresh per render so the `!Send` `Rc` action is born on the client, not captured by the island view.
fn keybinds() -> Config {
	Config {
		actions: vec![(
			Keybind { key: "s", alt: false, ctrl: false },
			std::rc::Rc::new(|s: &mut PackedState| {
				let json = s.save();
				let key = seed_key(s.breakpoint()).to_string();
				leptos::task::spawn_local(async move {
					if let Err(e) = save_layout(key, json).await {
						leptos::logging::error!("save_layout failed: {e}");
					}
				});
			}) as dockviewers::leptos::Action,
		)],
		..Default::default()
	}
}

#[cfg(feature = "ssr")]
fn layout_path(key: &str) -> PathBuf {
	v_utils::xdg_cache_dir!("dashboards").join(format!("layout-{key}.json"))
}

//REVIEW: server-file persistence lives here per-host; evaluate upstreaming into `dockviewers`
// (it ships a localStorage `persist` module) once this is proven — unify only if it removes real
// duplication versus the localStorage / SQLite hosts.
#[server]
async fn save_layout(key: String, json: String) -> Result<(), ServerFnError> {
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
