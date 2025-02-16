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
    v-utils.url = "github:valeratrades/.github";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, pre-commit-hooks, v-utils }:
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
          sassc # Native Sass compiler
          wasm-bindgen-cli
          binaryen # For wasm-opt
          nodePackages.tailwindcss
        ];

        buildTools = with pkgs; [
          mold-wrapped
          sccache
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
              trim-trailing-whitespace = {
                enable = true;
              };
            };
          };
        };
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        pname = manifest.name;
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

        workflowContents = import v-utils.ci { inherit pkgs; lastSupportedVersion = "nightly-2025-01-16"; jobsErrors = [ "rust-tests" ]; jobsWarnings = [ "rust-doc" "rust-clippy" "rust-machete" "rust-sort" "tokei" ]; };
        readme = (v-utils.readme-fw { inherit pkgs pname; lastSupportedVersion = "nightly-1.86"; rootDir = ./.; licenses = [{ name = "Blue Oak 1.0.0"; outPath = "LICENSE"; }]; badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ]; }).combined;
      in
      {
        devShells.default = pkgs.mkShell {
          inherit stdenv;
          nativeBuildInputs = [
            rustToolchain
            pkgs.openssl.dev
            pkgs.pkg-config
          ] ++ frontendTools ++ buildTools;

          shellHook =
            checks.pre-commit-check.shellHook +
            ''
              						rm -f ./.github/workflows/errors.yml; cp ${workflowContents.errors} ./.github/workflows/errors.yml
              						rm -f ./.github/workflows/warnings.yml; cp ${workflowContents.warnings} ./.github/workflows/warnings.yml

              						cp -f ${v-utils.files.licenses.blue_oak} ./LICENSE

              						cargo -Zscript -q ${v-utils.hooks.appendCustom} ./.git/hooks/pre-commit
              						cp -f ${(import v-utils.hooks.treefmt {inherit pkgs;})} ./.treefmt.toml
              						cp -f ${(import v-utils.hooks.preCommit) { inherit pkgs pname; }} ./.git/hooks/custom.sh

              						cp -f ${(import v-utils.files.rust.rustfmt {inherit pkgs;})} ./rustfmt.toml
              						cp -f ${(import v-utils.files.rust.deny {inherit pkgs;})} ./deny.toml
              						#cp -f ${(import v-utils.files.rust.config {inherit pkgs;})} ./.cargo/config.toml
              						cp -f ${(import v-utils.files.rust.toolchain {inherit pkgs;})} ./.cargo/rust-toolchain.toml
              						cp -f ${(import v-utils.files.gitignore) { inherit pkgs; langs = ["rs"];}} ./.gitignore

              						cp -f ${readme} ./README.md

              						# For Trunk to find sassc
              						export PATH="${pkgs.lib.makeBinPath frontendTools}:$PATH"
              						'';
          packages = with pkgs; [
            mold-wrapped
            pkg-config
            tailwindcss
          ];
        };
      }
    );
}
