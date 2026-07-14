use std::{collections::HashMap, sync::LazyLock, time::Duration};

use color_eyre::eyre::{Result, bail};
use futures::{
	lock::Mutex,
	stream::{self, StreamExt as _},
};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use v_exchanges::prelude::*;
use v_utils::trades::{Pair, Timeframe};

/// category10, cycled across highlighted (non-BTC) series
const PALETTE: [&str; 10] = ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf"];

/// One long-lived client, reused across every poll. The post-ban cooldown gate lives on the client
/// (`banned_until`), so rebuilding it each poll would forget the ban and keep re-hitting Binance —
/// which renews and escalates it. Reuse is what makes the gate actually stop the bleeding.
static BINANCE: LazyLock<Mutex<Box<dyn Exchange>>> = LazyLock::new(|| {
	let mut ex = ExchangeName::Binance.init_client();
	ex.set_retry_config(RetryConfig {
		max_retries: 3,
		..Default::default()
	});
	ex.set_timeout(Duration::from_secs(60));
	Mutex::new(ex)
});
/// Compact chart payload for the client-side Lightweight Charts shim.
/// Each series carries its own time domain, so late-listed pairs start where they list and
/// stocks/commodities keep their market-hours gaps — nothing is dropped or filled.
#[derive(Deserialize, Serialize)]
pub struct MarketStructureChart {
	bulk: Vec<Line>,          // grey background pairs: drawn as a single non-interactive canvas primitive
	values: Vec<Line>,        // per-series, parallel to `series`, in draw order
	series: Vec<SeriesMeta>,  // interactive highlights only: non-BTC highlights, then BTC (painted on top)
	legend: Vec<LegendEntry>, // legend order: top…, BTC (middle), …bottom
	title: String,
}
#[instrument(skip(exchange))]
pub async fn market_structure_json(limit: RequestRange, tf: Timeframe, exchange: &mut dyn Exchange, instrument: Instrument) -> Result<MarketStructureChart> {
	tracing::info!("Building market structure for {} {instrument} with limit={limit:?}, tf={tf:?}", exchange.name());

	let exch_info = match exchange.exchange_info(instrument).await {
		Ok(info) => info,
		Err(e) => {
			tracing::error!("Failed to fetch exchange info: {e:?}");
			return Err(e.into());
		}
	};

	let all_pairs = exch_info.usdt_pairs().collect::<Vec<Pair>>();
	tracing::info!("Found {} USDT pairs from exchange info", all_pairs.len());

	let closes = collect_data(&all_pairs, tf, limit, instrument, &*exchange).await?;

	let mut performance: Vec<(Pair, f64)> = closes.iter().map(|(k, v)| (*k, v.last().unwrap().1 - v.first().unwrap().1)).collect();
	performance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
	let n_samples = (performance.len() as f64).ln().round() as usize;
	let top: Vec<Pair> = performance.iter().rev().take(n_samples).map(|x| x.0).collect();
	let bottom: Vec<Pair> = performance.iter().take(n_samples).map(|x| x.0).collect();

	let line = |v: &[(i64, f64)]| Line {
		time: v.iter().map(|&(t, _)| t).collect(),
		value: v.iter().map(|&(_, x)| (x * 1e5).round() / 1e5).collect(),
	};

	let btc: Pair = "BTC-USDT".try_into().unwrap();
	let contains_btc = closes.contains_key(&btc);

	let label_for = |pair: Pair, symbol: Option<&str>| -> String {
		let p = performance.iter().find(|a| a.0 == pair).unwrap().1;
		let symbol = match symbol {
			Some(s) => s.to_string(),
			//HACK: should be formatted at this point. We want to always omit 1000, and the quote when it's USDT //TODO: unify this with LSR, etc
			None => {
				let s = pair.to_string();
				s[0..s.len() - 5].replace("1000", "")
			}
		};
		let sign = if p >= 0.0 { '+' } else { '-' };
		let change = format!("{:.2}", 100.0 * p.abs());
		format!("{symbol:<5}{sign}{change:>5}%")
	};

	// assign a stable palette color to each highlighted (non-BTC) pair, in legend order
	let mut color_of: HashMap<Pair, String> = HashMap::new();
	let mut legend = Vec::new();
	let mut palette_i = 0;
	let mut push_highlight = |pair: Pair, legend: &mut Vec<LegendEntry>, color_of: &mut HashMap<Pair, String>| {
		if pair == btc {
			return;
		}
		let color = PALETTE[palette_i % PALETTE.len()].to_string();
		palette_i += 1;
		color_of.insert(pair, color.clone());
		legend.push(LegendEntry {
			label: label_for(pair, None),
			color,
		});
	};
	for &pair in &top {
		push_highlight(pair, &mut legend, &mut color_of);
	}
	if contains_btc {
		color_of.insert(btc, "gold".to_string());
		legend.push(LegendEntry {
			label: label_for(btc, Some("~BTC~")),
			color: "gold".to_string(),
		});
	}
	for &pair in bottom.iter().rev() {
		push_highlight(pair, &mut legend, &mut color_of);
	}

	// greys never become interactive series — collected as `bulk` for the canvas primitive
	let mut bulk = Vec::new();
	for (pair, v) in &closes {
		if *pair == btc || color_of.contains_key(pair) {
			continue;
		}
		bulk.push(line(v));
	}

	// draw order: non-BTC highlights, then BTC last so it paints on top
	let mut series = Vec::new();
	let mut values = Vec::new();
	let highlight_order = top.iter().chain(bottom.iter().rev()).copied().filter(|p| *p != btc);
	let mut seen = std::collections::HashSet::new();
	for pair in highlight_order {
		if !seen.insert(pair) {
			continue;
		}
		series.push(SeriesMeta {
			pair: pair.to_string(),
			color: color_of[&pair].clone(),
			width: 2,
		});
		values.push(line(&closes[&pair]));
	}
	if contains_btc {
		series.push(SeriesMeta {
			pair: btc.to_string(),
			color: "gold".into(),
			width: 4,
		});
		values.push(line(&closes[&btc]));
	}

	let (mut lo, mut hi) = (i64::MAX, i64::MIN);
	for v in closes.values() {
		lo = lo.min(v.first().unwrap().0);
		hi = hi.max(v.last().unwrap().0);
	}
	let hours = (((hi - lo) + tf.duration().as_secs() as i64 * 2/*compensate for off-by-ones*/) as f64 / 3600.0).round() as i64;
	let title = format!("Last {hours}h of {}/{} pairs on {instrument}", closes.len(), all_pairs.len());

	Ok(MarketStructureChart {
		bulk,
		values,
		series,
		legend,
		title,
	})
}
/// Columnar {time, value}, kept as parallel arrays to stay small under gzip.
#[derive(Deserialize, Serialize)]
struct Line {
	time: Vec<i64>, // UNIX seconds, ascending
	value: Vec<f64>,
}
#[instrument(skip_all)]
async fn collect_data(pairs: &[Pair], tf: Timeframe, range: RequestRange, instrument: Instrument, exchange: &dyn Exchange) -> Result<HashMap<Pair, Vec<(i64, f64)>>> {
	tracing::info!("Starting data collection for {} pairs with tf={tf:?}, range={range:?}", pairs.len());

	let progress = crate::dashboards::_core::Progress::of::<MarketStructureChart>();
	progress.set_target(pairs.len());

	//HACK: assumes we're never misaligned here
	// ponytail: fixed cap keeps the per-pair burst under Binance's 2400 weight/min IP limit; tune if the universe grows
	const CONCURRENCY: usize = 12;
	let results: Vec<_> = stream::iter(pairs.to_vec().into_iter().map(|pair| {
		let symbol = Symbol::new(pair, instrument);
		async move {
			let r = match get_historical_data(symbol, tf, range, exchange).await {
				Ok(series) => {
					tracing::debug!("Successfully fetched {} data points for {symbol}", series.col_closes.len());
					Ok((symbol, series))
				}
				Err(e) => {
					tracing::warn!("Failed to fetch data for symbol: {symbol}. Error: {e:?}");
					Err(e)
				}
			};
			progress.inc();
			r
		}
	}))
	.buffer_unordered(CONCURRENCY)
	.collect()
	.await;
	let mut normalized: HashMap<Pair, Vec<(i64, f64)>> = HashMap::new();
	let mut failed_pairs = Vec::new();
	let mut btc_found = false;

	results.into_iter().for_each(|result| match result {
		Ok((symbol, series)) => {
			if "BTC-USDT" == symbol.pair {
				btc_found = true;
			}
			let mut pts: Vec<(i64, f64)> = series.col_open_times.iter().zip(&series.col_closes).map(|(t, c)| (t.as_second(), *c)).collect();
			pts.sort_by_key(|p| p.0);
			match pts.first() {
				// normalize each series to its own first close, in log-return space
				Some(&(_, first)) => {
					pts.iter_mut().for_each(|p| p.1 = (p.1 / first).ln());
					normalized.insert(symbol.pair, pts);
				}
				None => eprintln!("Received empty data for: {symbol}"),
			}
		}
		Err(e) => failed_pairs.push(format!("{e:?}")),
	});

	let success_rate = (normalized.len() as f64 / pairs.len() as f64) * 100.0;
	let threshold = if success_rate < 70.0 { " - below 70% threshold" } else { "" };
	tracing::info!("Fetched {} pairs, {} failed ({success_rate:.1}% success rate{threshold})", normalized.len(), failed_pairs.len());

	if !btc_found {
		tracing::error!(
			"BTC-USDT missing from results. attempted: {}, succeeded: {}, failed: {}",
			pairs.len(),
			normalized.len(),
			failed_pairs.len()
		);
		bail!("Failed to fetch data for BTC-USDT, aborting. Check logs for individual fetch errors.");
	}

	Ok(normalized)
}
#[allow(unused)]
#[derive(Clone, Debug, Default, derive_new::new)]
//HACK: manual roll of a DataFrame. No checks for alignment (or proper vectorization, for that matter)...
struct RelevantHistoricalData {
	col_open_times: Vec<Timestamp>,
	col_opens: Vec<f64>,
	col_highs: Vec<f64>,
	col_lows: Vec<f64>,
	col_closes: Vec<f64>,
	col_volumes: Vec<f64>,
}
#[instrument(skip_all, fields(symbol = %symbol))]
async fn get_historical_data(symbol: Symbol, tf: Timeframe, range: RequestRange, exchange: &dyn Exchange) -> Result<RelevantHistoricalData> {
	tracing::debug!("Fetching klines for {symbol} (tf={tf:?}, range={range:?})");

	let klines = match exchange.klines(symbol, tf, range).await {
		Ok(k) => {
			tracing::debug!("Received {} klines for {symbol}", k.len());
			k
		}
		Err(e) => {
			tracing::error!("Failed to fetch klines for {symbol}: {e:?}");
			return Err(e.into());
		}
	};

	let mut open_time = Vec::new();
	let mut open = Vec::new();
	let mut high = Vec::new();
	let mut low = Vec::new();
	let mut close = Vec::new();
	let mut volume = Vec::new();
	for k in klines {
		open_time.push(k.open_time);
		open.push(k.open);
		high.push(k.high);
		low.push(k.low);
		close.push(k.close);
		volume.push(k.volume_quote);
	}
	Ok(RelevantHistoricalData {
		col_open_times: open_time,
		col_opens: open,
		col_highs: high,
		col_lows: low,
		col_closes: close,
		col_volumes: volume,
	})
}
#[derive(Deserialize, Serialize)]
struct SeriesMeta {
	pair: String,
	color: String,
	width: u8,
}
#[derive(Deserialize, Serialize)]
struct LegendEntry {
	label: String,
	color: String,
}

impl crate::dashboards::_core::SourceData for MarketStructureChart {
	fn decay_horizon() -> Timeframe {
		"30m".into()
	}

	async fn fetch() -> Result<Self> {
		let tf = "5m".into();
		let range = (24 * 12 + 1).into(); // 24h, given `5m` tf
		let mut exchange = BINANCE.lock().await;
		market_structure_json(range, tf, &mut **exchange, Instrument::Perp).await
	}

	fn fmt_progress(loaded: usize, target: usize) -> String {
		format!("pulled {loaded}/{target} pairs")
	}
}

//TODO!!!: provide additional information: 1) BTCDOM, 2) average, 3) correlation, 4) volatility
