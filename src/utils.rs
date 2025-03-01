use std::{fs, path::PathBuf};
use v_utils::xdg_cache;

#[cfg(feature = "ssr")]
pub trait Mock
where
	Self: Sized + serde::de::DeserializeOwned + serde::Serialize, {
	fn __name() -> &'static str {
		let type_path = std::any::type_name::<Self>();
		type_path.split("::").last().unwrap_or(type_path)
	}
	fn __fpath() -> PathBuf {
		xdg_cache!("dashboards").join(format!("{}.json", Self::__name()))
	}
	fn persist(&self) -> std::io::Result<()> {
		tracing::info!("Persisting current {}", Self::__name());
		let json = serde_json::to_string_pretty(self)?;
		fs::write(Self::__fpath(), json)?;
		Ok(())
	}
	fn load_mock() -> std::io::Result<Self> {
		let filepath = Self::__fpath();
		let json = fs::read_to_string(&filepath)?;
		let contents = serde_json::from_str(&json)?;
		tracing::info!("Loaded mock from {}", filepath.display());
		Ok(contents)
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
