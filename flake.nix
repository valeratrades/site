{
  description = "Minimal Leptos development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/23e89b7da85c3640bbc2173fe04f4bd114342367";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
		pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    workflow-parts.url = "github:valeratrades/.github?dir=.github/workflows/nix-parts";
    hooks.url = "github:valeratrades/.github?dir=hooks";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, pre-commit-hooks, workflow-parts, hooks, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        # Minimal Rust toolchain with just what we need
        rustToolchain = with fenix.packages.${system}; combine [
          latest.cargo
          latest.rustc
          latest.rust-std
          latest.rust-src
          targets.wasm32-unknown-unknown.latest.rust-std
        ];

        # Frontend tools needed for Leptos development
        frontendTools = with pkgs; [
          trunk
          sassc  # Native Sass compiler
          wasm-bindgen-cli
          binaryen # For wasm-opt
          nodePackages.tailwindcss
        ];
			
				buildTools = with pkgs; [
					mold-wrapped
					openssl
					pkg-config
				];

				checks = {
					pre-commit-check = pre-commit-hooks.lib.${system}.run {
						src = ./.;
						hooks = {
							treefmt = {
								enable = true;
								settings = {
									#BUG: this option does NOTHING
									fail-on-change = false; # that's GHA's job, pre-commit hooks stricty *do*
									formatters = with pkgs; [
										nixpkgs-fmt
									];
								};
							};
						};
					};
				};
				workflowContents = (import ./.github/workflows/ci.nix) { inherit pkgs workflow-parts; };
				stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;
      in
      {
        devShells.default = pkgs.mkShell {
					inherit stdenv;
          nativeBuildInputs = [
            rustToolchain
          ] ++ frontendTools ++ buildTools;

          shellHook = 
						checks.pre-commit-check.shellHook +
						''
						rm -f ./.github/workflows/errors.yml; cp ${workflowContents.errors} ./.github/workflows/errors.yml
						rm -f ./.github/workflows/warnings.yml; cp ${workflowContents.warnings} ./.github/workflows/warnings.yml

						cargo -Zscript -q ${hooks.appendCustom} ./.git/hooks/pre-commit
						cp -f ${(import hooks.treefmt { inherit pkgs; })} ./.treefmt.toml

            # For Trunk to find sassc
            export PATH="${pkgs.lib.makeBinPath frontendTools}:$PATH"
          '';
        };
      }
    );
}
