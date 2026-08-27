//! What a text panel needs to show itself unclipped, counted rather than tuned. The whole site is
//! `font-family: monospace` and every text panel renders a `<pre>`, so the smallest unwrapped box is
//! exactly *longest line × line count* — a number the panel already holds.
//!
//! `deck.rs` collects these per panel and pushes them into the live layout as the content arrives,
//! which is why the seed carries no hand-measured minimums.

use dockviewers::leptos::MinSize;

/// Width of one monospace character cell, in em. DejaVu/Liberation/Menlo are all ≈0.6; Consolas
/// 0.55.
// ponytail: the calibration knob for the horizontal floor — if panels stop one column short, this is
// the number to raise, not the per-panel counts.
const CH_REM: f64 = 0.6;
/// Height of one `<pre>` line, in rem. Pinned by `.dv-host pre { line-height: 1.25; font-size: 1rem }`
/// in `public/custom.css` — change one and the other is wrong.
// ponytail: the vertical twin of `CH_REM`.
const LINE_REM: f64 = 1.25;

/// The smallest box that shows something unclipped, in monospace character cells. Composes, so a
/// panel built of parts is its parts' extents stacked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extent {
	pub cols: u32,
	pub rows: u32,
}

impl Extent {
	/// Longest line × line count, skipping `<…>` spans: the LSR outlier block goes through
	/// `inner_html` and embeds a coloured `<span>`, whose markup occupies no columns.
	pub fn of(text: &str) -> Self {
		Self {
			cols: text.lines().map(visible_len).max().unwrap_or(0),
			rows: text.lines().count() as u32,
		}
	}

	/// `self` stacked on top of `other`.
	pub fn above(self, o: Self) -> Self {
		Self {
			cols: self.cols.max(o.cols),
			rows: self.rows + o.rows,
		}
	}

	pub fn pad(self, cols: u32, rows: u32) -> Self {
		Self {
			cols: self.cols + cols,
			rows: self.rows + rows,
		}
	}

	/// The layout floor this extent implies, with the tile's title bar added on top (dockviewers
	/// insets the chrome band out of the content slot, so the content's rows need it back).
	pub fn min_size(self, title_h_rem: f64) -> MinSize {
		MinSize::Rem {
			w: self.cols as f64 * CH_REM,
			h: self.rows as f64 * LINE_REM + title_h_rem,
		}
	}
}

fn visible_len(line: &str) -> u32 {
	let mut n = 0;
	let mut in_tag = false;
	for c in line.chars() {
		match (c, in_tag) {
			('<', false) => in_tag = true,
			('>', true) => in_tag = false,
			(_, false) => n += 1,
			_ => {}
		}
	}
	n
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn counts_the_widest_visible_line() {
		assert_eq!(Extent::of("ab\nabcd\nabc"), Extent { cols: 4, rows: 3 });
		assert_eq!(Extent::of(""), Extent { cols: 0, rows: 0 });

		// The shape `Lsrs::display_outliers` emits: markup that renders as nothing must not be counted,
		// or the panel floors a whole `<span style="...">` wider than it draws.
		let colored = "Collected for <span style=\"color: #f59e0b;\">42</span>/380 pairs";
		assert_eq!(Extent::of(colored).cols, "Collected for 42/380 pairs".len() as u32);
	}

	#[test]
	fn stacking_takes_the_wider_and_the_taller_total() {
		let a = Extent { cols: 10, rows: 2 };
		let b = Extent { cols: 4, rows: 3 };
		assert_eq!(a.above(b), Extent { cols: 10, rows: 5 });
		assert_eq!(a.above(b).pad(3, 2), Extent { cols: 13, rows: 7 });
	}
}
