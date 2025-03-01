use std::fs;
use v_utils::xdg_cache;

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
		tracing::info!("Persisting current {}", Self::__name());
		let json = serde_json::to_string_pretty(self)?;
		tracing::debug!(?json);
		fs::write(xdg_cache!("").join(format!("{}.json", Self::__name())), json)?;
		Ok(())
	}
	fn load_mock() -> std::io::Result<Self> {
		let json = fs::read_to_string(xdg_cache!("").join(format!("{}.json", Self::__name())))?;
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
