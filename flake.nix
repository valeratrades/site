{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/23e89b7da85c3640bbc2173fe04f4bd114342367";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    v-utils.url = "github:valeratrades/.github";
  };

  outputs =
    { self
    , nixpkgs
    , nixpkgs-unstable
    , flake-utils
    , rust-overlay
    , pre-commit-hooks
    , v-utils
    ,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };
        pkgs-unstable = import nixpkgs-unstable {
          inherit system;
        };

        frontendTools = with pkgs; [
          sassc # Native Sass compiler
          #wasm-bindgen-cli #NB: substituted by manually installing v100 via cargo
          binaryen # For wasm-opt
          pkgs-unstable.typst # For blog .typ -> .html compilation (needs 0.14+ for HTML export)
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
          jobsOther = [ "loc-badge" ];
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

              # Copy the current directory and manually link dependencies
              src = pkgs.lib.cleanSource ./.;

              cargoLock = {
                lockFile = ./Cargo.lock;
                outputHashes = {
                  "leptos-routable-0.2.0" = "sha256-w17sr9fLbUaCHP6x/fVSmR5dYduTGBlXBDna7Ksq+ZM=";
                };
              };

              buildInputs = with pkgs; [
                openssl.dev
              ];

              nativeBuildInputs = with pkgs; [
                pkg-config
                tailwindcss
                wasm-bindgen-cli
              ] ++ frontendTools;

              # Build tailwind CSS before cargo build
              preBuild = sourceTailwind;

              # Custom build phase: build server binary and WASM client separately
              buildPhase = ''
                runHook preBuild

                # Build the server binary with SSR feature
                echo "Building server binary..."
                cargo build --release --bin ${pname} --features ssr --no-default-features

                # Build the WASM client with hydrate feature
                echo "Building WASM client..."
                cargo build --release --lib --target wasm32-unknown-unknown --features hydrate --no-default-features

                # Run wasm-bindgen to generate JS glue code
                echo "Running wasm-bindgen..."
                mkdir -p target/site/pkg
                wasm-bindgen --target web \
                  --out-dir target/site/pkg \
                  --out-name ${pname} \
                  target/wasm32-unknown-unknown/release/${pname}.wasm

                # Optimize WASM with wasm-opt
                echo "Optimizing WASM..."
                wasm-opt -Oz target/site/pkg/${pname}_bg.wasm -o target/site/pkg/${pname}_bg.wasm

                runHook postBuild
              '';

              # Install the binary and site assets
              installPhase = ''
                                runHook preInstall

                                mkdir -p $out/bin
                                mkdir -p $out/share/${pname}/pkg

                                # Copy the server binary
                                cp target/release/${pname} $out/bin/${pname}

                                # Copy the WASM and JS assets
                                cp -r target/site/pkg/* $out/share/${pname}/pkg/

                                # Copy CSS
                                mkdir -p $out/share/${pname}/style
                                cp style/tailwind_out.css $out/share/${pname}/style/

                                # Copy public assets if they exist
                                if [ -d public ]; then
                                  cp -r public/* $out/share/${pname}/
                                fi

                                # Create a wrapper script that sets LEPTOS_SITE_ROOT
                                cat > $out/bin/${pname}-wrapped <<EOF
                #!/bin/sh
                export LEPTOS_SITE_ROOT="$out/share/${pname}"
                exec "$out/bin/${pname}" "\$@"
                EOF
                                chmod +x $out/bin/${pname}-wrapped

                                # Make the wrapped version the default
                                rm $out/bin/${pname}
                                mv $out/bin/${pname}-wrapped $out/bin/${pname}

                                runHook postInstall
              '';

              doCheck = false; # Skip tests in build
              auditable = false; # Disable cargo-auditable (doesn't support edition 2024)
            };
          };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/${pname}";
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

          shellHook =
            pre-commit-check.shellHook +
            workflowContents.shellHook +
            ''
              							cp -f ${v-utils.files.licenses.blue_oak} ./LICENSE

              							cargo -Zscript -q ${v-utils.hooks.appendCustom} ./.git/hooks/pre-commit
              							cp -f ${(v-utils.hooks.treefmt) { inherit pkgs; }} ./.treefmt.toml
              							cp -f ${(v-utils.hooks.preCommit) { inherit pkgs pname; }} ./.git/hooks/custom.sh

              							#mkdir -p ./.cargo
              							#cp -f ${(v-utils.files.rust.config { inherit pkgs; })} ./.cargo/config.toml
              							cp -f ${(v-utils.files.rust.rustfmt { inherit pkgs; })} ./rustfmt.toml
              							cp -f ${
                       (v-utils.files.gitignore {
                         inherit pkgs;
                         langs = [ "rs" ];
                       })
                     } ./.gitignore

              							cp -f ${readme} ./README.md

                            # cargo-leptos must match leptos crate's version; thus can't install through nixpkgs
                            # Use binstall on Ubuntu (faster), cargo install elsewhere
                            if grep -qi ubuntu /etc/os-release 2>/dev/null; then
                              cargo binstall -y cargo-leptos
                            else
                              cargo install cargo-leptos
                            fi

              							${sourceTailwind}
            '';
          env.RUSTFLAGS = "-Zmacro-backtrace"; # XXX: would be overriding existing RUSTFLAGS
          #env.LEPTOS_WASM_BINDGEN_VERSION = "0.2.106"; #NB: must be in sync with `leptos` crate's version. Suggestion of `-f` wasm-bindgen install in their error is wrong, - this is how you actually do it.

          packages = with pkgs; [
            mold-wrapped
            openssl
            pkg-config
            sccache
            rust
          ];
        };
      }
    );
}
