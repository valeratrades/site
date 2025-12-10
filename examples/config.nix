{
	mock = true;
	admin = {
		users = {
			admin = "100%";
			valeratrades = "100%";
			prof = "50%"; #dbg
		};
		creds = {
			"claude_token" = {
				env = "CLAUDE_TOKEN";
			};
			"windows_uni_headless_cmd" = "./uni_headless.exe --visible --username 'vasakharov' --auto-submit -a --password 'TODO'";
			"windows_uni_headless_get" = ''Invoke-WebRequest -Uri "https://github.com/valeratrades/uni_headless/releases/download/latest-windows/uni_headless.exe" -OutFile "uni_headless.exe"'';
		};
	};
}
