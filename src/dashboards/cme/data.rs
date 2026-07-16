use color_eyre::eyre::{Result, eyre};
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use v_utils::{NowThen, PrettyPrint};

#[allow(unused)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, derive_new::new)]
pub struct Positions {
	long: PositionsInfo,
	short: PositionsInfo,
	spreading: PositionsInfo,
}
#[allow(unused)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, derive_new::new)]
pub struct CftcReport {
	pub date: Timestamp,
	pub dealer_intermidiary: Positions,
	pub asset_manager_or_institutional: Positions,
	pub leveraged_funds: Positions,
	pub other_reportables: Positions,
}
impl CftcReport {
	pub fn to_markdown_table(&self) -> String {
		let format_num = |n: f64| format!("{n:.0}");
		let format_pct = |n: f64| format!("{n:.1}");
		let format_trader = |n: Option<u32>| n.map_or(".".to_string(), |v| v.to_string());

		let date_str = self.date.strftime("%B %d, %Y").to_string();

		format!(
			"# Traders in Financial Futures - Futures Only Positions as of {}\n\n\
           |Position Type|Dealer Intermediary|||Asset Manager/Institutional|||Leveraged Funds|||Other Reportables|||\n\
           |------------|-------------------|---|---|----------------------|---|---|--------------|---|---|-----------------|---|---|\n\
           ||Long|Short|Spread|Long|Short|Spread|Long|Short|Spread|Long|Short|Spread|\n\
           |**Current**|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|\n\
           |**Changes**|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|\n\
           |**% of Open**|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|\n\
           |**Traders**|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|",
			date_str,
			// Current positions
			format_num(self.dealer_intermidiary.long.current),
			format_num(self.dealer_intermidiary.short.current),
			format_num(self.dealer_intermidiary.spreading.current),
			format_num(self.asset_manager_or_institutional.long.current),
			format_num(self.asset_manager_or_institutional.short.current),
			format_num(self.asset_manager_or_institutional.spreading.current),
			format_num(self.leveraged_funds.long.current),
			format_num(self.leveraged_funds.short.current),
			format_num(self.leveraged_funds.spreading.current),
			format_num(self.other_reportables.long.current),
			format_num(self.other_reportables.short.current),
			format_num(self.other_reportables.spreading.current),
			// Changes
			format_num(self.dealer_intermidiary.long.change_since_last_week),
			format_num(self.dealer_intermidiary.short.change_since_last_week),
			format_num(self.dealer_intermidiary.spreading.change_since_last_week),
			format_num(self.asset_manager_or_institutional.long.change_since_last_week),
			format_num(self.asset_manager_or_institutional.short.change_since_last_week),
			format_num(self.asset_manager_or_institutional.spreading.change_since_last_week),
			format_num(self.leveraged_funds.long.change_since_last_week),
			format_num(self.leveraged_funds.short.change_since_last_week),
			format_num(self.leveraged_funds.spreading.change_since_last_week),
			format_num(self.other_reportables.long.change_since_last_week),
			format_num(self.other_reportables.short.change_since_last_week),
			format_num(self.other_reportables.spreading.change_since_last_week),
			// Percentages
			format_pct(self.dealer_intermidiary.long.percent_of_open),
			format_pct(self.dealer_intermidiary.short.percent_of_open),
			format_pct(self.dealer_intermidiary.spreading.percent_of_open),
			format_pct(self.asset_manager_or_institutional.long.percent_of_open),
			format_pct(self.asset_manager_or_institutional.short.percent_of_open),
			format_pct(self.asset_manager_or_institutional.spreading.percent_of_open),
			format_pct(self.leveraged_funds.long.percent_of_open),
			format_pct(self.leveraged_funds.short.percent_of_open),
			format_pct(self.leveraged_funds.spreading.percent_of_open),
			format_pct(self.other_reportables.long.percent_of_open),
			format_pct(self.other_reportables.short.percent_of_open),
			format_pct(self.other_reportables.spreading.percent_of_open),
			// Traders
			format_trader(self.dealer_intermidiary.long.number_of_traders),
			format_trader(self.dealer_intermidiary.short.number_of_traders),
			format_trader(self.dealer_intermidiary.spreading.number_of_traders),
			format_trader(self.asset_manager_or_institutional.long.number_of_traders),
			format_trader(self.asset_manager_or_institutional.short.number_of_traders),
			format_trader(self.asset_manager_or_institutional.spreading.number_of_traders),
			format_trader(self.leveraged_funds.long.number_of_traders),
			format_trader(self.leveraged_funds.short.number_of_traders),
			format_trader(self.leveraged_funds.spreading.number_of_traders),
			format_trader(self.other_reportables.long.number_of_traders),
			format_trader(self.other_reportables.short.number_of_traders),
			format_trader(self.other_reportables.spreading.number_of_traders),
		)
	}
}

// www.cftc.gov's HTML report sits behind a Cloudflare bot-challenge that 403s datacenter/pod egress
// IPs; the Socrata data API serves the same "Traders in Financial Futures" report as JSON with no challenge.
pub async fn fetch_cftc_positions() -> Result<CftcReport> {
	let rows: Vec<TffRow> = reqwest::Client::new()
		.get("https://publicreporting.cftc.gov/resource/gpe5-46if.json")
		.query(&[("cftc_contract_market_code", CFTC_CODE_BTC), ("$order", "report_date_as_yyyy_mm_dd DESC"), ("$limit", "1")])
		.send()
		.await?
		.json()
		.await?;
	let row = rows.into_iter().next().ok_or_else(|| eyre!("CFTC API returned no rows for BTC ({CFTC_CODE_BTC})"))?;
	row.try_into()
}
static CFTC_CODE_BTC: &str = "133741";

/// One "Traders in Financial Futures - Futures Only" report row, as served by the CFTC Socrata API
/// (dataset gpe5-46if). Every numeric column arrives as a JSON string; spread-trader counts are
/// omitted for all categories but leveraged funds.
#[serde_as]
#[derive(Deserialize)]
struct TffRow {
	report_date_as_yyyy_mm_dd: String,

	#[serde_as(as = "DisplayFromStr")]
	dealer_positions_long_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	dealer_positions_short_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	dealer_positions_spread_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	asset_mgr_positions_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	asset_mgr_positions_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	asset_mgr_positions_spread: f64,
	#[serde_as(as = "DisplayFromStr")]
	lev_money_positions_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	lev_money_positions_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	lev_money_positions_spread: f64,
	#[serde_as(as = "DisplayFromStr")]
	other_rept_positions_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	other_rept_positions_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	other_rept_positions_spread: f64,

	#[serde_as(as = "DisplayFromStr")]
	change_in_dealer_long_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_dealer_short_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_dealer_spread_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_asset_mgr_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_asset_mgr_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_asset_mgr_spread: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_lev_money_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_lev_money_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_lev_money_spread: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_other_rept_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_other_rept_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	change_in_other_rept_spread: f64,

	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_dealer_long_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_dealer_short_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_dealer_spread_all: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_asset_mgr_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_asset_mgr_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_asset_mgr_spread: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_lev_money_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_lev_money_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_lev_money_spread: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_other_rept_long: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_other_rept_short: f64,
	#[serde_as(as = "DisplayFromStr")]
	pct_of_oi_other_rept_spread: f64,

	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_dealer_long_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_dealer_short_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_asset_mgr_long_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_asset_mgr_short_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_lev_money_long_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_lev_money_short_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_lev_money_spread: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_other_rept_long_all: Option<u32>,
	#[serde_as(as = "Option<DisplayFromStr>")]
	#[serde(default)]
	traders_other_rept_short: Option<u32>,
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, derive_new::new)]
struct PositionsInfo {
	current: f64,
	change_since_last_week: f64,
	percent_of_open: f64,
	number_of_traders: Option<u32>,
}
impl TryFrom<TffRow> for CftcReport {
	type Error = color_eyre::eyre::Report;

	fn try_from(r: TffRow) -> Result<Self> {
		let day = r.report_date_as_yyyy_mm_dd.split('T').next().unwrap_or(&r.report_date_as_yyyy_mm_dd);
		let naive_date = Date::strptime("%Y-%m-%d", day).map_err(|e| eyre!("Failed to parse date `{day}`: {e}"))?;
		let eastern = TimeZone::get("America/New_York").map_err(|e| eyre!("Failed to load Eastern timezone: {e}"))?;
		let date = naive_date
			.at(15, 30, 0, 0)
			.to_zoned(eastern)
			.map_err(|e| eyre!("Failed to create Eastern timezone datetime: {e}"))?
			.timestamp();

		Ok(CftcReport {
			date,
			dealer_intermidiary: Positions::new(
				PositionsInfo::new(r.dealer_positions_long_all, r.change_in_dealer_long_all, r.pct_of_oi_dealer_long_all, r.traders_dealer_long_all),
				PositionsInfo::new(
					r.dealer_positions_short_all,
					r.change_in_dealer_short_all,
					r.pct_of_oi_dealer_short_all,
					r.traders_dealer_short_all,
				),
				PositionsInfo::new(r.dealer_positions_spread_all, r.change_in_dealer_spread_all, r.pct_of_oi_dealer_spread_all, None),
			),
			asset_manager_or_institutional: Positions::new(
				PositionsInfo::new(r.asset_mgr_positions_long, r.change_in_asset_mgr_long, r.pct_of_oi_asset_mgr_long, r.traders_asset_mgr_long_all),
				PositionsInfo::new(
					r.asset_mgr_positions_short,
					r.change_in_asset_mgr_short,
					r.pct_of_oi_asset_mgr_short,
					r.traders_asset_mgr_short_all,
				),
				PositionsInfo::new(r.asset_mgr_positions_spread, r.change_in_asset_mgr_spread, r.pct_of_oi_asset_mgr_spread, None),
			),
			leveraged_funds: Positions::new(
				PositionsInfo::new(r.lev_money_positions_long, r.change_in_lev_money_long, r.pct_of_oi_lev_money_long, r.traders_lev_money_long_all),
				PositionsInfo::new(
					r.lev_money_positions_short,
					r.change_in_lev_money_short,
					r.pct_of_oi_lev_money_short,
					r.traders_lev_money_short_all,
				),
				PositionsInfo::new(
					r.lev_money_positions_spread,
					r.change_in_lev_money_spread,
					r.pct_of_oi_lev_money_spread,
					r.traders_lev_money_spread,
				),
			),
			other_reportables: Positions::new(
				PositionsInfo::new(
					r.other_rept_positions_long,
					r.change_in_other_rept_long,
					r.pct_of_oi_other_rept_long,
					r.traders_other_rept_long_all,
				),
				PositionsInfo::new(
					r.other_rept_positions_short,
					r.change_in_other_rept_short,
					r.pct_of_oi_other_rept_short,
					r.traders_other_rept_short,
				),
				PositionsInfo::new(r.other_rept_positions_spread, r.change_in_other_rept_spread, r.pct_of_oi_other_rept_spread, None),
			),
		})
	}
}
impl std::fmt::Display for CftcReport {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let institutional_change = {
			let institutional = &self.asset_manager_or_institutional;
			DirectionalPositionsChange {
				name: "Institutional",
				long: NowThen::from_now_diff(institutional.long.current, institutional.long.change_since_last_week),
				short: NowThen::from_now_diff(institutional.short.current, institutional.short.change_since_last_week),
			}
		};
		let funds_change = {
			let funds = &self.leveraged_funds;
			DirectionalPositionsChange {
				name: "Hedgefunds",
				long: NowThen::from_now_diff(funds.long.current, funds.long.change_since_last_week),
				short: NowThen::from_now_diff(funds.short.current, funds.short.change_since_last_week),
			}
		};
		write!(
			f,
			"CFTC Report as of {}:\n- {}\n- {}",
			self.date,
			institutional_change.to_string_pretty(2),
			funds_change.to_string_pretty(2)
		)
	}
}

#[derive(Clone, Copy, Debug, Default, derive_new::new)]
struct DirectionalPositionsChange {
	name: &'static str,
	long: NowThen,
	short: NowThen,
}
impl PrettyPrint for DirectionalPositionsChange {
	fn pretty(&self, f: &mut std::fmt::Formatter<'_>, root_indent: u8) -> std::fmt::Result {
		writeln!(f, "{}:", self.name)?;
		let indent = " ".repeat(root_indent as usize);
		write!(f, "{indent}Long: {}, Short: {}", self.long, self.short)
	}
}
