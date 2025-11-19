extern crate clap;

#[derive(Debug, Clone, v_utils::macros::MyConfigPrimitives)]
#[cfg_attr(feature = "ssr", derive(v_utils::macros::Settings))]
pub struct Settings {
	pub mock: Option<bool>,
}

impl Default for Settings {
	fn default() -> Self {
		Self { mock: Some(false) }
	}
}

impl Settings {
	pub fn mock(&self) -> bool {
		self.mock.unwrap_or(false)
	}
}
