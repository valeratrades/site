{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/23e89b7da85c3640bbc2173fe04f4bd114342367";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    v-utils.url = "github:valeratrades/.github";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      pre-commit-hooks,
      v-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };

        frontendTools = with pkgs; [
          sassc # Native Sass compiler
          #wasm-bindgen-cli #NB: substituted by manually installing v100 via cargo
          binaryen # For wasm-opt
        ];

        rust = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "rust-docs"
              "rustc-codegen-cranelift-preview"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          }
        );

        buildTools = with pkgs; [
          mold-wrapped
          sccache
          openssl
          pkg-config
          tailwindcss
          #flyctl # might end up using it for deployment
        ];

        sourceTailwind = ''tailwindcss -i ./style/tailwind_in.css -o ./style/tailwind_out.css '';

        pre-commit-check = pre-commit-hooks.lib.${system}.run (v-utils.files.preCommit { inherit pkgs; });
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        pname = manifest.name;
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

        workflowContents = v-utils.ci {
          inherit pkgs;
          lastSupportedVersion = "nightly-2025-01-16";
          jobsErrors = [ "rust-tests" ];
          jobsWarnings = [
            "rust-doc"
            "rust-clippy"
            "rust-machete"
            "rust-sorted"
            "tokei"
          ];
        };
        readme = v-utils.readme-fw {
          inherit pkgs pname;
          lastSupportedVersion = "nightly-1.86";
          rootDir = ./.;
          licenses = [
            {
              name = "Blue Oak 1.0.0";
              outPath = "LICENSE";
            }
          ];
          badges = [
            "msrv"
            "crates_io"
            "docs_rs"
            "loc"
            "ci"
          ];
        };
      in
      {
        #TODO: actually implement build process (ref: https://book.leptos.dev/deployment/ssr.html)
        #TODO; figure out what's the equivalent of docker's `EXPOSE 8080`
        packages =
          let
            rustc = rust;
            cargo = rust;
            rustPlatform = pkgs.makeRustPlatform {
              inherit rustc cargo stdenv;
            };
          in
          {
            default = rustPlatform.buildRustPackage rec {
              inherit pname;
              version = manifest.version;

              preBuild = sourceTailwind ++ ''
                							mkdir ./build/app
                							cp -r ./target/site/ ./build/app/site
                							cp ./target/release/${pname} ./build/app/
                						'';
              buildInputs = with pkgs; [
                openssl.dev
              ];
              nativeBuildInputs = with pkgs; [ pkg-config ];

              cargoLock.lockFile = ./Cargo.lock;
              src = pkgs.lib.cleanSource ./.;
            };
          };

        devShells.default = pkgs.mkShell {
          inherit stdenv;
          nativeBuildInputs = [
            #rustToolchain
            pkgs.openssl.dev
            pkgs.pkg-config
          ]
          ++ frontendTools
          ++ buildTools;

          #env = {
          #  LEPTOS_SASS_VERSION = "1.71.0";
          #};

          shellHook = pre-commit-check.shellHook + ''
            							mkdir -p ./.github/workflows
            							rm -f ./.github/workflows/errors.yml; cp ${workflowContents.errors} ./.github/workflows/errors.yml
            							rm -f ./.github/workflows/warnings.yml; cp ${workflowContents.warnings} ./.github/workflows/warnings.yml

            							cp -f ${v-utils.files.licenses.blue_oak} ./LICENSE

            							cargo -Zscript -q ${v-utils.hooks.appendCustom} ./.git/hooks/pre-commit
            							cp -f ${(v-utils.hooks.treefmt) { inherit pkgs; }} ./.treefmt.toml
            							cp -f ${(v-utils.hooks.preCommit) { inherit pkgs pname; }} ./.git/hooks/custom.sh

            							#mkdir -p ./.cargo
            							#cp -f ${(v-utils.files.rust.config { inherit pkgs; })} ./.cargo/config.toml
            							cp -f ${(v-utils.files.rust.rustfmt { inherit pkgs; })} ./rustfmt.toml
            							cp -f ${(v-utils.files.rust.deny { inherit pkgs; })} ./deny.toml
            							cp -f ${
                     (v-utils.files.gitignore {
                       inherit pkgs;
                       langs = [ "rs" ];
                     })
                   } ./.gitignore

            							cp -f ${readme} ./README.md

            							alias lw="cargo leptos watch --hot-reload"

            							${sourceTailwind}
          '';

          packages = with pkgs; [
            mold-wrapped
            openssl
            #cargo-leptos #TODO: figure out if I need this, or if `cargo leptos` is the more correct way to use it (that is guaranteed to stay up-to-date)
            #NB: with what I currently understand, atm need to install cargo-leptos through cargo exclusively, as I need absolute latest
            pkg-config
            sccache
            rust
          ];
        };
      }
    );
}
