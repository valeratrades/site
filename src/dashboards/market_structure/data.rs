use std::{collections::HashMap, sync::Arc, time::Duration};

use color_eyre::eyre::{Result, bail};
use futures::future::join_all;
use jiff::Timestamp;
use plotly::{Plot, Scatter, common::Line};
use tracing::instrument;
use v_exchanges::prelude::*;
use v_utils::trades::{Pair, Timeframe};

#[instrument]
pub async fn try_build(limit: RequestRange, tf: Timeframe, exchange_name: ExchangeName, instrument: Instrument) -> Result<Plot> {
	tracing::info!("Building market structure for {} {} with limit={:?}, tf={:?}", exchange_name, instrument, limit, tf);

	let mut exchange = exchange_name.init_client();
	exchange.set_max_tries(3);
	exchange.set_recv_window(Duration::from_secs(60));

	tracing::debug!("Fetching exchange info for {}", instrument);
	let exch_info = match exchange.exchange_info(instrument, None).await {
		Ok(info) => info,
		Err(e) => {
			tracing::error!("Failed to fetch exchange info: {:?}", e);
			return Err(e.into());
		}
	};

	let all_usdt_pairs = exch_info.usdt_pairs().collect::<Vec<Pair>>();
	tracing::info!("Found {} USDT pairs from exchange info", all_usdt_pairs.len());

	// Check if BTC-USDT is in the list
	let has_btc_usdt = all_usdt_pairs.iter().any(|p| p.to_string() == "BTC-USDT");
	tracing::info!("BTC-USDT in pairs list: {}", has_btc_usdt);

	let (normalized_df, dt_index) = collect_data(&all_usdt_pairs, tf, limit, instrument, Arc::new(exchange)).await?;
	Ok(plotly_closes(normalized_df, dt_index, tf, instrument, &all_usdt_pairs))
}

#[instrument(skip_all)]
pub async fn collect_data(pairs: &[Pair], tf: Timeframe, range: RequestRange, instrument: Instrument, exchange: Arc<Box<dyn Exchange>>) -> Result<(HashMap<Pair, Vec<f64>>, Vec<Timestamp>)> {
	tracing::info!("Starting data collection for {} pairs with tf={:?}, range={:?}", pairs.len(), tf, range);

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

	tracing::info!("Successfully fetched data for {} pairs, {} pairs failed", data.len(), failed_pairs.len());

	if dt_index.is_empty() {
		if !btc_usdt_found {
			tracing::error!(
				"BTC-USDT was not in the successful results. Total pairs attempted: {}, succeeded: {}, failed: {}",
				pairs.len(),
				data.len(),
				failed_pairs.len()
			);
			tracing::error!("BTC-USDT present in pairs list: {}", pairs.iter().any(|p| p.to_string() == "BTC-USDT"));
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
			eprintln!("misaligned: {key}");
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
	tracing::debug!("Fetching klines for {} (tf={:?}, range={:?})", symbol, tf, range);

	let klines = match exchange.klines(symbol, tf, range, None).await {
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
	let title = format!("Last {hours}h of {}/{} pairs on {instrument}", normalized_closes.len(), all_pairs.len());
	plot.set_layout(plotly::Layout::new().title(title));

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
