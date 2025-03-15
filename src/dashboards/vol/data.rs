use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{Result, eyre};
use reqwest::Client;
use serde_json::{Value, json};
use v_utils::NowThen;

///// Fetch VIX volatility index data
//pub async fn vix(client: &Client, api_key: &str, comparison_limit: u64) -> Result<f64> {
//    let quote_url = format!("https://api.twelvedata.com/quote?symbol=VIX:CBOE&apikey={}", api_key);
//    let response = client.get(&quote_url).send().await?;
//    let quote: Value = response.json().await?;
//
//    if !comparison_limit.eq(&0) {
//        let history_url = format!(
//            "https://api.twelvedata.com/time_series?symbol=VIX:CBOE&interval=1h&outputsize={}&apikey={}",
//            comparison_limit + 1,
//            api_key
//        );
//
//        let timestamp = quote["timestamp"].as_str()
//            .ok_or_else(|| eyre!("Invalid timestamp"))?.parse::<u64>()?;
//        let vix_close = quote["close"].as_str()
//            .ok_or_else(|| eyre!("Invalid close value"))?.parse::<f64>()?;
//
//        let hours_ago = (SystemTime::now()
//            .duration_since(UNIX_EPOCH)?.as_secs() - timestamp) as f64 / 3600.0;
//
//        if hours_ago > comparison_limit as f64 {
//            return Ok(vix_close);
//        }
//
//        let history_response = client.get(&history_url).send().await?;
//        let history: Value = history_response.json().await?;
//
//        let points = history["values"].as_array()
//            .ok_or_else(|| eyre!("Failed to get VIX history"))?;
//
//        let now = Utc::now();
//        let then = now - Duration::hours(comparison_limit as i64);
//
//        let mut closest_point = &points[0];
//        let mut min_distance = i64::MAX;
//
//        for point in points {
//            let dt_str = point["datetime"].as_str()
//                .ok_or_else(|| eyre!("Invalid datetime"))?;
//            let dt = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S")?;
//            let dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
//
//            let distance = (then - dt).num_seconds().abs();
//            if distance < min_distance {
//                min_distance = distance;
//                closest_point = point;
//            }
//        }
//
//        let closest_value = closest_point["close"].as_str()
//            .ok_or_else(|| eyre!("Invalid close value"))?.parse::<f64>()?;
//
//        return Ok(closest_value);
//    }
//
//    let vix_close = quote["close"].as_str()
//        .ok_or_else(|| eyre!("Invalid close value"))?.parse::<f64>()?;
//
//    Ok(vix_close)
//}
//
///// Fetch BitMEX volatility index data
//pub async fn bvol(client: &Client, comparison_limit: u64) -> Result<f64> {
//    let url = format!(
//        "https://www.bitmex.com/api/v1/trade?symbol=.BVOL24H&count={}&reverse=true",
//        comparison_limit * 12 + 1
//    );
//
//    let response = client.get(&url).send().await?;
//    let trades: Vec<Value> = response.json().await?;
//
//    if trades.is_empty() {
//        return Err(eyre!("No BVOL data received"));
//    }
//
//    // Get current price
//    let current_price = trades[0]["price"].as_f64()
//        .ok_or_else(|| eyre!("Invalid price data"))?;
//
//    if comparison_limit.eq(&0) || trades.len() <= 1 {
//        return Ok(current_price);
//    }
//
//    // Get historical price for comparison
//    let historical_price = trades.last().unwrap()["price"].as_f64()
//        .ok_or_else(|| eyre!("Invalid historical price data"))?;
//
//    Ok(historical_price)
//}

pub async fn bvol(duration: Duration) -> Result<NowThen> {
	if duration.as_secs() / 3600 > 24 {
		return Err(eyre!("Duration must be less than 24h"));
	} else if duration.as_secs() / 3600 < 10 {
		return Err(eyre!("Duration must be greater than 10m"));
	}

	let bm = v_exchanges::bitmex::Bitmex::default();
	let bvols = bm.bvol(duration).await.unwrap();
	let (now, then) = (bvols.last().unwrap().price, bvols.first().unwrap().price);

	Ok(NowThen::new(now, then).add_duration(duration))
}

pub async fn vix(duration: Duration) -> Result<NowThen> {
	if duration.as_secs() / 3600 > 24 {
		return Err(eyre!("Duration must be less than 24 hours"));
	} else if duration.as_secs() / 3600 <= 2 {
		return Err(eyre!("Duration must be at least 2h"));
	}

	let hours_floored = (duration.as_secs() / 3600) as u8; //NB: relies on previous check that should (for other reasons) ensure that this is a valid u8
	v_exchanges::yahoo::vix_change("1h".into(), hours_floored).await
}
