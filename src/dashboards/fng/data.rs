use chrono::{/*serde::ts_seconds, */ DateTime, Utc};
use color_eyre::eyre::{Result, bail, eyre};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug, Serialize, Deserialize)]
pub struct FngResponse {
	name: String,
	data: Vec<FngRaw>,
	metadata: Metadata,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct FngRaw {
	#[serde_as(as = "DisplayFromStr")]
	value: f64,
	value_classification: String,
	#[serde_as(as = "DisplayFromStr")]
	#[serde(rename = "timestamp")]
	ts_seconds: i64,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(rename = "time_until_update")]
	seconds_until_update: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
	error: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Fng {
	pub value: f64,
	pub timestamp: DateTime<Utc>,
}

/// Is expected to be most often used wtih `limit=1`, so not exposing [Duration](std::time::Duration) instead of it.
pub async fn btc_fngs_hourly(limit: usize) -> Result<Vec<Fng>> {
	let client = Client::new();
	let response = client
		.get(format!("https://api.alternative.me/fng/?limit={limit}"))
		.send()
		.await
		.map_err(|e| eyre!("Failed to fetch Fear and Greed Index: {}", e))?;

	if !response.status().is_success() {
		bail!("Failed to fetch Fear and Greed Index. Status: {}", response.status());
	}

	let fng_response: FngResponse = response.json().await.map_err(|e| eyre!("Failed to parse Fear and Greed Index response: {e}"))?;
	if let Some(error) = fng_response.metadata.error {
		if !error.is_empty() {
			bail!("API returned an error: {error}");
		}
	}
	fng_response
		.data
		.into_iter()
		.map(|raw| -> Result<Fng> {
			let timestamp = DateTime::from_timestamp(raw.ts_seconds, 0).ok_or_else(|| eyre!("Invalid timestamp: {}", raw.ts_seconds))?;

			Ok(Fng { value: raw.value, timestamp })
		})
		.collect()
}
