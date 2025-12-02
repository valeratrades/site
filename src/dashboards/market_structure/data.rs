use std::{collections::HashMap, sync::Arc, time::Duration};

use color_eyre::eyre::{Result, bail};
use futures::future::join_all;
use jiff::Timestamp;
use plotly::{
	Plot, Scatter,
	common::{Line, Title},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use v_exchanges::prelude::*;
use v_utils::{
	trades::{Pair, Timeframe},
	xdg_state_file,
};

/// Persisted market structure data with metadata for cache invalidation
#[derive(Clone, Debug, Deserialize, Serialize)]
struct PersistedMarketData {
	/// When this data was collected
	timestamp: std::time::SystemTime,
	/// Normalized data (pair -> log-normalized closes)
	normalized_df: HashMap<Pair, Vec<f64>>,
	/// Time index from BTC-USDT
	dt_index: Vec<Timestamp>,
	/// Parameters used to collect this data
	params: CollectionParams,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct CollectionParams {
	timeframe: Timeframe,
	range_debug: String,
	instrument_debug: String,
}

impl CollectionParams {
	fn new(timeframe: Timeframe, range: RequestRange, instrument: Instrument) -> Self {
		Self {
			timeframe,
			range_debug: format!("{:?}", range),
			instrument_debug: format!("{:?}", instrument),
		}
	}
}

impl PersistedMarketData {
	fn cache_file(params: &CollectionParams) -> std::path::PathBuf {
		let filename = format!(
			"market_structure_{}_{:?}_{}.json",
			params.instrument_debug,
			params.timeframe,
			params.range_debug.replace([':', ' ', '{', '}'], "_")
		);
		xdg_state_file!(filename)
	}

	fn try_load(params: &CollectionParams, max_age: Duration) -> Option<Self> {
		let cache_file = Self::cache_file(params);

		let metadata = std::fs::metadata(&cache_file).ok()?;
		let age = metadata.modified().ok()?.elapsed().ok()?;

		if age > max_age {
			tracing::info!("Cache file exists but is too old (age: {:?}, max: {:?})", age, max_age);
			return None;
		}

		let json = std::fs::read_to_string(&cache_file).ok()?;
		let data: Self = serde_json::from_str(&json).ok()?;

		// Verify params match
		if data.params != *params {
			tracing::warn!("Cache params mismatch, ignoring cached data");
			return None;
		}

		tracing::info!("Loaded market structure data from cache (age: {:?})", age);
		Some(data)
	}

	fn save(&self) -> Result<()> {
		let cache_file = Self::cache_file(&self.params);
		let json = serde_json::to_string_pretty(self)?;
		std::fs::write(&cache_file, json)?;
		tracing::info!("Saved market structure data to {:?}", cache_file);
		Ok(())
	}
}

#[instrument]
pub async fn try_build(limit: RequestRange, tf: Timeframe, exchange_name: ExchangeName, instrument: Instrument) -> Result<Plot> {
	tracing::info!("Building market structure for {} {} with limit={:?}, tf={:?}", exchange_name, instrument, limit, tf);

	let mut exchange = exchange_name.init_client();
	exchange.set_max_tries(3);
	exchange.set_timeout(Duration::from_secs(60));

	tracing::debug!("Fetching exchange info for {instrument}");
	let exch_info = match exchange.exchange_info(instrument).await {
		Ok(info) => info,
		Err(e) => {
			tracing::error!("Failed to fetch exchange info: {:?}", e);
			return Err(e.into());
		}
	};

	let all_usdt_pairs = exch_info.usdt_pairs().collect::<Vec<Pair>>();
	tracing::info!("Found {} USDT pairs from exchange info", all_usdt_pairs.len());

	// Check if BTC-USDT is in the list
	let has_btc_usdt = all_usdt_pairs.iter().any(|p| "BTC-USDT" == *p);
	tracing::info!("BTC-USDT in pairs list: {}", has_btc_usdt);

	let (normalized_df, dt_index) = collect_data(&all_usdt_pairs, tf, limit, instrument, Arc::new(exchange)).await?;
	Ok(plotly_closes(normalized_df, dt_index, tf, instrument, &all_usdt_pairs))
}

#[instrument(skip_all)]
pub async fn collect_data(pairs: &[Pair], tf: Timeframe, range: RequestRange, instrument: Instrument, exchange: Arc<Box<dyn Exchange>>) -> Result<(HashMap<Pair, Vec<f64>>, Vec<Timestamp>)> {
	tracing::info!("Starting data collection for {} pairs with tf={:?}, range={:?}", pairs.len(), tf, range);

	let params = CollectionParams::new(tf, range, instrument);

	// Calculate max age based on timeframe - reload cycle should be proportional to the timeframe
	// For 1h data, reload every hour; for 1d data, reload every day, etc.
	let max_age = tf.duration();

	// Try to load from cache first
	if let Some(cached) = PersistedMarketData::try_load(&params, max_age) {
		tracing::info!("Using cached market structure data");
		return Ok((cached.normalized_df, cached.dt_index));
	}

	tracing::info!("No valid cache found, fetching fresh data");

	//HACK: assumes we're never misaligned here
	let futures = pairs.iter().map(|pair| {
		let exchange = Arc::clone(&exchange);
		let symbol = Symbol::new(*pair, instrument);
		async move {
			match get_historical_data(symbol, tf, range, exchange).await {
				Ok(series) => {
					tracing::debug!("Successfully fetched {} data points for {}", series.col_closes.len(), symbol);
					Ok((symbol, series))
				}
				Err(e) => {
					tracing::warn!("Failed to fetch data for symbol: {symbol}. Error: {e:?}");
					Err(e)
				}
			}
		}
	});

	let results = join_all(futures).await;
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
				failed_pairs.push(format!("{:?}", e));
			}
		}
	});

	let total_pairs = pairs.len();
	let success_rate = (data.len() as f64 / total_pairs as f64) * 100.0;
	if success_rate < 70.0 {
		tracing::warn!(
			"Successfully fetched data for {} pairs, {} pairs failed ({:.1}% success rate - below 70% threshold)",
			data.len(),
			failed_pairs.len(),
			success_rate
		);
	} else {
		tracing::info!(
			"Successfully fetched data for {} pairs, {} pairs failed ({:.1}% success rate)",
			data.len(),
			failed_pairs.len(),
			success_rate
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

	// Persist the collected data to cache
	let persisted_data = PersistedMarketData {
		timestamp: std::time::SystemTime::now(),
		normalized_df: normalized_df.clone(),
		dt_index: dt_index.clone(),
		params,
	};

	if let Err(e) = persisted_data.save() {
		tracing::warn!("Failed to save market structure data to cache: {:?}", e);
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
	tracing::debug!("Fetching klines for {} (tf={:?}, range={:?})", symbol, tf, range);

	let klines = match exchange.klines(symbol, tf, range).await {
		Ok(k) => {
			tracing::debug!("Received {} klines for {}", k.len(), symbol);
			k
		}
		Err(e) => {
			tracing::error!("Failed to fetch klines for {}: {:?}", symbol, e);
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

//TODO!!!: provide additional information: 1) BTCDOM, 2) average, 3) correlation, 4) volatility
pub fn plotly_closes(normalized_closes: HashMap<Pair, Vec<f64>>, dt_index: Vec<Timestamp>, tf: Timeframe, instrument: Instrument, all_pairs: &[Pair]) -> Plot {
	let mut performance: Vec<(Pair, f64)> = normalized_closes.iter().map(|(k, v)| (*k, (v[v.len() - 1] - v[0]))).collect();
	performance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

	let n_samples = (performance.len() as f64).ln().round() as usize;
	let top: Vec<Pair> = performance.iter().rev().take(n_samples).map(|x| x.0).collect();
	let bottom: Vec<Pair> = performance.iter().take(n_samples).map(|x| x.0).collect();

	let mut plot = Plot::new();
	let hours = (dt_index.first().unwrap().duration_since(*dt_index.last().unwrap()) + tf.signed_duration() * 2/*compensate for off-by-ones*/)
		.as_hours()
		.abs();

	// Calculate success rate and color only the numerator if below 70%
	let success_rate = (normalized_closes.len() as f64 / all_pairs.len() as f64) * 100.0;
	let title_text = if success_rate < 70.0 {
		format!(
			"Last {hours}h of <span style=\"color: #f59e0b;\">{}</span>/{} pairs on {instrument}",
			normalized_closes.len(),
			all_pairs.len()
		)
	} else {
		format!("Last {hours}h of {}/{} pairs on {instrument}", normalized_closes.len(), all_pairs.len())
	};
	plot.set_layout(plotly::Layout::new().title(Title::with_text(&title_text)));

	let mut add_trace = |name: Pair, width: f64, color: Option<&'static str>, legend: Option<String>| {
		let y_values: Vec<f64> = normalized_closes.get(&name).unwrap().to_owned();
		let x_values: Vec<String> = dt_index.iter().map(|dt| dt.to_string()).collect();

		let mut line = Line::new().width(width);
		if let Some(c) = color {
			line = line.color(c);
		}

		let mut trace = Scatter::new(x_values, y_values).mode(plotly::common::Mode::Lines).line(line);

		if let Some(l) = legend {
			trace = trace.name(&l);
		} else {
			trace = trace.show_legend(false);
		}
		plot.add_trace(trace);
	};

	let mut contains_btcusdt = false;
	for col_name in normalized_closes.keys() {
		if &col_name.to_string() == "BTC-USDT" {
			contains_btcusdt = true;
			continue;
		}
		if top.contains(col_name) || bottom.contains(col_name) {
			continue;
		}
		add_trace(*col_name, 1.0, Some("grey"), None);
	}
	let mut labeled_trace = |col_name: Pair, symbol: Option<&str>, line_width: f64, color: Option<&'static str>| {
		let p: f64 = performance.iter().find(|a| a.0 == col_name).unwrap().1;
		let symbol = match symbol {
			Some(s) => s,
			None => &col_name.to_string()[0..col_name.to_string().len() - 5].to_string().replace("1000", ""), //HACK: should be formatted at this point. We want to always omit 1000, and the quote when it's USDT //TODO: unify this with LSR, etc
		};
		let sign = if p >= 0.0 { '+' } else { '-' };
		let change = format!("{:.2}", 100.0 * p.abs());
		let legend = format!("{:<5}{}{:>5}%", symbol, sign, change);
		add_trace(col_name, line_width, color, Some(legend));
	};
	for col_name in top.into_iter() {
		labeled_trace(col_name, None, 2.0, None);
	}
	if contains_btcusdt {
		labeled_trace("BTC-USDT".try_into().unwrap(), Some("~BTC~"), 3.5, Some("gold"));
	}
	for col_name in bottom.into_iter().rev() {
		labeled_trace(col_name, None, 2.0, None);
	}

	plot
}
