use std::fs;

#[cfg(feature = "ssr")]
use v_utils::prelude::*;

//TODO: switch to v_utils
#[cfg(feature = "ssr")]
pub fn share_dir() -> std::path::PathBuf {
	let base_path = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", std::env::var("HOME").unwrap()));
	let share_dir = std::path::PathBuf::from(base_path).join(env!("CARGO_PKG_NAME"));
	std::fs::create_dir_all(&share_dir).unwrap();
	share_dir
}

#[cfg(feature = "ssr")]
pub trait Mock
where
	Self: Sized + serde::de::DeserializeOwned + serde::Serialize, {
	fn __name() -> &'static str {
		static NAME: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
		NAME.get_or_init(|| {
			let type_path = std::any::type_name::<Self>();
			type_path.split("::").last().unwrap_or(type_path)
		})
	}
	fn persist(&self) -> std::io::Result<()> {
		info!("Persisting current {}", Self::__name());
		let json = serde_json::to_string_pretty(self)?;
		debug!(?json);
		fs::write(xdg_cache("").join(format!("{}.json", Self::__name())), json)?;
		Ok(())
	}
	fn load_mock() -> std::io::Result<Self> {
		let json = fs::read_to_string(share_dir().join(format!("{}.json", Self::__name())))?;
		Ok(serde_json::from_str(&json)?)
	}
}

///```rs
///crate::try_load_mock!(MarketStructure, .into());
///```
#[macro_export]
macro_rules! try_load_mock {
	// Basic case without transform
	($r#type:path) => {
		if use_context::<Settings>().expect("Settings not found in context").mock {
			match <$type>::load_mock() {
				Ok(v) => return Ok(v),
				Err(e) => tracing::warn!("Couldn't load mock: {:?}\n-> Falling back to requesting new data.", e),
			}
		}
		tracing::trace!("mock didn't happen, requesting new data");
	};

	// With transform
	($type:path; $($transform:tt)+) => {
		if use_context::<Settings>().expect("Settings not found in context").mock {
			match <$type>::load_mock() {
				Ok(v) => {
					return Ok(v$($transform)+);
				}
				Err(e) => tracing::warn!("Couldn't load mock: {:?}\n-> Falling back to requesting new data.", e),
			}
		}
		tracing::trace!("mock didn't happen, requesting new data");
	};
}
