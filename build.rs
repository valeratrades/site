fn main() {
	println!("cargo:rerun-if-changed=.git/HEAD");
	println!("cargo:rerun-if-changed=.git/refs/");
	println!("cargo:rerun-if-env-changed=SITE_BUILD_REV");
	// Hermetic (nix) builds have no `.git`; the flake passes the commit in SITE_BUILD_REV.
	let hash = std::env::var("SITE_BUILD_REV").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| {
		std::process::Command::new("git")
			.args(["rev-parse", "--short", "HEAD"])
			.output()
			.ok()
			.filter(|o| o.status.success())
			.and_then(|o| String::from_utf8(o.stdout).ok())
			.map(|s| s.trim().to_string())
			.unwrap_or_else(|| "unknown".into())
	});
	println!("cargo:rustc-env=GIT_HASH={hash}");
}
