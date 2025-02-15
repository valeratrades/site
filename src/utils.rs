use std::fs;

use serde::{Serialize, de::DeserializeOwned};
use tracing::{debug, info};

pub fn share_dir() -> std::path::PathBuf {
	let base_path = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", std::env::var("HOME").unwrap()));
	let share_dir = std::path::PathBuf::from(base_path).join(env!("CARGO_PKG_NAME"));
	std::fs::create_dir_all(&share_dir).unwrap();
	share_dir
}

pub trait Mock
where
	Self: Sized + DeserializeOwned + Serialize, {
	const NAME: &'static str;
	fn persist(&self) -> std::io::Result<()> {
		info!("Persisting current {}", Self::NAME);
		let json = serde_json::to_string_pretty(self)?;
		debug!(?json);
		fs::write(share_dir().join(format!("{}.json", Self::NAME)), json)?;
		Ok(())
	}
	fn load_mock() -> std::io::Result<Self> {
		let json = fs::read_to_string(share_dir().join(format!("{}.json", Self::NAME)))?;
		Ok(serde_json::from_str(&json)?)
	}
}
