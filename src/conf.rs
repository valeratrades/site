extern crate clap;

#[derive(Debug, Clone, Default, v_utils::macros::MyConfigPrimitives)]
#[cfg_attr(feature = "ssr", derive(v_utils::macros::Settings))]
pub struct Settings {
	pub mock: bool,
}
