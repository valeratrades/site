use std::{collections::HashMap, sync::Arc, time::Duration};

use color_eyre::eyre::{Result, bail};
use futures::stream::{self, StreamExt as _};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use v_exchanges::prelude::*;
use v_utils::trades::{Pair, Timeframe};

/// category10, cycled across highlighted (non-BTC) series
const PALETTE: [&str; 10] = ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf"];
/// Collected market data: normalized closes per pair and the time index
pub type MarketData = (HashMap<Pair, Vec<f64>>, Vec<Timestamp>);

/// Compact chart payload for the client-side Lightweight Charts shim.
/// Columnar (parallel arrays, no per-point keys) to keep the JSON small under gzip.
#[derive(Deserialize, Serialize)]
pub struct MarketStructureChart {
	time: Vec<i64>,           // UNIX seconds, ascending
	bulk: Vec<Vec<f64>>,      // grey background pairs: drawn as a single non-interactive canvas primitive
	values: Vec<Vec<f64>>,    // per-series, parallel to `series`, in draw order
	series: Vec<SeriesMeta>,  // interactive highlights only: non-BTC highlights, then BTC (painted on top)
	legend: Vec<LegendEntry>, // legend order: top…, BTC (middle), …bottom
	title: String,
}
#[instrument]
pub async fn market_structure_json(limit: RequestRange, tf: Timeframe, exchange_name: ExchangeName, instrument: Instrument) -> Result<MarketStructureChart> {
	tracing::info!("Building market structure for {exchange_name} {instrument} with limit={limit:?}, tf={tf:?}");

	let mut exchange = exchange_name.init_client();
	exchange.set_retry_config(RetryConfig {
		max_retries: 3,
		..Default::default()
	});
	exchange.set_timeout(Duration::from_secs(60));

	let exch_info = match exchange.exchange_info(instrument).await {
		Ok(info) => info,
		Err(e) => {
			tracing::error!("Failed to fetch exchange info: {e:?}");
			return Err(e.into());
		}
	};

	let all_pairs = exch_info.usdt_pairs().collect::<Vec<Pair>>();
	tracing::info!("Found {} USDT pairs from exchange info", all_pairs.len());

	let (normalized_df, dt_index) = collect_data(&all_pairs, tf, limit, instrument, Arc::new(exchange)).await?;

	// collect_data returns the un-aligned frame; drop any series whose length disagrees with the
	// time index so `values[i]` indexing below is always in bounds.
	let closes: HashMap<Pair, Vec<f64>> = normalized_df.into_iter().filter(|(_, v)| v.len() == dt_index.len()).collect();

	let mut performance: Vec<(Pair, f64)> = closes.iter().map(|(k, v)| (*k, v[v.len() - 1] - v[0])).collect();
	performance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
	let n_samples = (performance.len() as f64).ln().round() as usize;
	let top: Vec<Pair> = performance.iter().rev().take(n_samples).map(|x| x.0).collect();
	let bottom: Vec<Pair> = performance.iter().take(n_samples).map(|x| x.0).collect();

	// time index, sorted ascending; `order` re-indexes every series to match
	let mut order: Vec<usize> = (0..dt_index.len()).collect();
	order.sort_by_key(|&i| dt_index[i].as_second());
	let time: Vec<i64> = order.iter().map(|&i| dt_index[i].as_second()).collect();
	let column = |v: &[f64]| -> Vec<f64> { order.iter().map(|&i| (v[i] * 1e5).round() / 1e5).collect() };

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
		bulk.push(column(v));
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
		values.push(column(&closes[&pair]));
	}
	if contains_btc {
		series.push(SeriesMeta {
			pair: btc.to_string(),
			color: "gold".into(),
			width: 4,
		});
		values.push(column(&closes[&btc]));
	}

	let span_secs = time.last().unwrap() - time.first().unwrap();
	let hours = ((span_secs + tf.duration().as_secs() as i64 * 2/*compensate for off-by-ones*/) as f64 / 3600.0).round() as i64;
	let title = format!("Last {hours}h of {}/{} pairs on {instrument}", closes.len(), all_pairs.len());

	Ok(MarketStructureChart {
		time,
		bulk,
		values,
		series,
		legend,
		title,
	})
}
#[instrument(skip_all)]
pub async fn collect_data(pairs: &[Pair], tf: Timeframe, range: RequestRange, instrument: Instrument, exchange: Arc<Box<dyn Exchange>>) -> Result<MarketData> {
	tracing::info!("Starting data collection for {} pairs with tf={tf:?}, range={range:?}", pairs.len());

	let progress = crate::dashboards::_core::Progress::of::<MarketStructureChart>();
	progress.set_target(pairs.len());

	//HACK: assumes we're never misaligned here
	// ponytail: fixed cap keeps the per-pair burst under Binance's 2400 weight/min IP limit; tune if the universe grows
	const CONCURRENCY: usize = 12;
	let results: Vec<_> = stream::iter(pairs.to_vec().into_iter().map(|pair| {
		let exchange = Arc::clone(&exchange);
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
	let mut data: HashMap<Pair, Vec<f64>> = HashMap::new();
	let mut dt_index = Vec::new();
	let mut failed_pairs = Vec::new();
	let mut btc_usdt_found = false;

	results.into_iter().for_each(|result| {
		match result {
			Ok((symbol, series)) => {
				if "BTC-USDT" == symbol.pair {
					btc_usdt_found = true;
					dt_index = series.col_open_times.clone();
					tracing::info!("BTC-USDT data fetched successfully with {} data points", series.col_closes.len());
				}
				data.insert(symbol.pair, series.col_closes);
			}
			Err(e) => {
				// Track which pairs failed - the symbol info is lost here, but we logged it above
				failed_pairs.push(format!("{e:?}"));
			}
		}
	});

	let total_pairs = pairs.len();
	let success_rate = (data.len() as f64 / total_pairs as f64) * 100.0;
	if success_rate < 70.0 {
		tracing::warn!(
			"Successfully fetched data for {} pairs, {} pairs failed ({success_rate:.1}% success rate - below 70% threshold)",
			data.len(),
			failed_pairs.len()
		);
	} else {
		tracing::info!(
			"Successfully fetched data for {} pairs, {} pairs failed ({success_rate:.1}% success rate)",
			data.len(),
			failed_pairs.len()
		);
	}

	if dt_index.is_empty() {
		if !btc_usdt_found {
			tracing::error!(
				"BTC-USDT was not in the successful results. Total pairs attempted: {}, succeeded: {}, failed: {}",
				pairs.len(),
				data.len(),
				failed_pairs.len()
			);
			tracing::error!("BTC-USDT present in pairs list: {}", pairs.iter().any(|p| "BTC-USDT" == *p));
			bail!("Failed to fetch data for BTC-USDT, aborting. Check logs for individual fetch errors.");
		} else {
			tracing::error!("BTC-USDT was found but dt_index is empty - this shouldn't happen!");
			bail!("BTC-USDT data appears corrupted (empty time index)");
		}
	}

	let mut normalized_df: HashMap<Pair, Vec<f64>> = HashMap::new();
	for (symbol, closes) in data.into_iter() {
		let first_close: f64 = match closes.first() {
			Some(v) => *v,
			None => {
				eprintln!("Received empty data for: {symbol}");
				continue;
			}
		};
		let normalized = closes.into_iter().map(|p| (p / first_close).ln()).collect();
		normalized_df.insert(symbol, normalized);
	}

	let mut aligned_df = normalized_df.clone();
	for (key, closes) in normalized_df.iter() {
		if closes.len() != dt_index.len() {
			//HACK: maybe we want to fill the missing fields instead if there are not many of them
			tracing::warn!("misaligned: {key}");
			aligned_df.remove(key).unwrap();
		}
	}

	Ok((normalized_df, dt_index))
}
#[allow(unused)]
#[derive(Clone, Debug, Default, derive_new::new)]
//HACK: manual roll of a DataFrame. No checks for alignment (or proper vectorization, for that matter)...
pub struct RelevantHistoricalData {
	col_open_times: Vec<Timestamp>,
	col_opens: Vec<f64>,
	col_highs: Vec<f64>,
	col_lows: Vec<f64>,
	col_closes: Vec<f64>,
	col_volumes: Vec<f64>,
}
#[instrument(skip_all, fields(symbol = %symbol))]
pub async fn get_historical_data(symbol: Symbol, tf: Timeframe, range: RequestRange, exchange: Arc<Box<dyn Exchange>>) -> Result<RelevantHistoricalData> {
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
		market_structure_json(range, tf, ExchangeName::Binance, Instrument::Perp).await
	}

	fn fmt_progress(loaded: usize, target: usize) -> String {
		format!("pulled {loaded}/{target} pairs")
	}
}

//TODO!!!: provide additional information: 1) BTCDOM, 2) average, 3) correlation, 4) volatility
