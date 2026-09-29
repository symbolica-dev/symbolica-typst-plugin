{
  description = "Symbolica computer algebra and integration for Typst";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      packageVersion = (builtins.fromTOML (builtins.readFile ./typst.toml)).package.version;
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      typstWithPackages = pkgs: pkgs.typst.withPackages (packages: [
        packages.cetz_0_5_2 packages.cetz-plot_0_1_4
        packages.parsely_0_1_1 packages.tidy_0_4_3
      ]);
      typstCheckScript = ''
        for namespace in preview local; do
          package_dir="$check_dir/packages/$namespace/symbolica"
          mkdir -p "$package_dir"
          ln -s "$PWD" "$package_dir/${packageVersion}"
        done
        export TYPST_PACKAGE_PATH="$check_dir/packages"
        python3 scripts/readme-examples.py
        for example in basic showcase expression-grid lotka-volterra phase-portrait integration; do
          typst compile --root . "symbolica/examples/$example.typ" "$check_dir/$example.pdf"
        done
        for fixture in api-surface parsing literals parsely-metadata attachments expression-tree worked-examples integration; do
          typst compile --root . "symbolica/tests/$fixture.typ" "$check_dir/test-$fixture.pdf"
        done
        python3 symbolica/tests/literals-check.py
        python3 symbolica/tests/parsing-check.py
        typst compile --creation-timestamp 0 --root . symbolica/manual.typ "$check_dir/manual.pdf"
        if ! cmp -s symbolica/manual.pdf "$check_dir/manual.pdf"; then
          echo "symbolica/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
          exit 1
        fi
        for example in local-package integration-local-package; do
          typst compile --package-path "$check_dir/packages" --root . \
            "symbolica/examples/$example.typ" "$check_dir/$example.pdf"
        done
        rm -rf "$check_dir/packages"
      '';
      packageScript = ''
        python3 scripts/prepare-distribution.py
        python3 scripts/readme-examples.py --package-root "dist/universe/packages/preview/symbolica/${packageVersion}"
      '';
    in {
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          TYPST_FONT_PATHS = "${pkgs.dejavu_fonts}/share/fonts";
          TYPST_IGNORE_SYSTEM_FONTS = "true";
          packages = [ pkgs.binaryen pkgs.cargo pkgs.lld pkgs.rustc pkgs.rustfmt pkgs.python3 pkgs.stdenv.cc (typstWithPackages pkgs) ];
        };
      });
      apps = eachSystem (pkgs:
        let
          path = [ pkgs.binaryen pkgs.cargo pkgs.coreutils pkgs.diffutils pkgs.gnutar pkgs.gzip pkgs.lld pkgs.rustc pkgs.python3 pkgs.stdenv.cc (typstWithPackages pkgs) ];
          app = name: text: {
            type = "app";
            program = "${pkgs.writeShellApplication {
              inherit name;
              runtimeInputs = path;
              text = ''
                export TYPST_FONT_PATHS="${pkgs.dejavu_fonts}/share/fonts"
                export TYPST_IGNORE_SYSTEM_FONTS=true
              '' + text;
            }}/bin/${name}";
            meta.description = "Run ${name}";
          };
          engineBuildScript = builtins.readFile ./scripts/build-engine.sh;
          dependencyBoundaryScript = ''
            for features in wasm wasm,plugin; do
              tree="$(cargo tree --edges normal --prefix none --format '{p}' --package symbolica-typst-plugin --no-default-features --features "$features")"
              for dependency in spenso idenso; do
                if [[ "$tree" == "$dependency v"* || "$tree" == *$'\n'"$dependency v"* ]]; then
                  echo "symbolica-typst-plugin must not depend on $dependency" >&2
                  exit 1
                fi
              done
            done
            tree="$(cargo tree --edges normal --prefix none --format '{p}' --package symbolica-typst-plugin --no-default-features --features wasm)"
            for dependency in symbolica-integrate wasm-minimal-protocol; do
              if [[ "$tree" == *"$dependency v"* ]]; then
                echo "The reusable library must not depend on $dependency" >&2
                exit 1
              fi
            done
            # Check a real library build: `cargo check` alone misses accidental
            # cdylib linking that would require the consuming plugin's imports.
            cargo build --locked --lib --target wasm32-unknown-unknown \
              --package symbolica-typst-plugin --no-default-features --features wasm
          '';
        in rec {
          default = build;
          build = app "symbolica-build" engineBuildScript;
          build-engine = build;
          package = app "symbolica-package" (engineBuildScript + packageScript);
          source-package = app "symbolica-source-package" ''python3 scripts/prepare-source.py'';
          manual = app "symbolica-manual" (engineBuildScript + ''
            manual_packages="$(mktemp -d)"
            trap 'rm -rf "$manual_packages"' EXIT
            mkdir -p "$manual_packages/preview/symbolica"
            ln -s "$PWD" "$manual_packages/preview/symbolica/${packageVersion}"
            export TYPST_PACKAGE_PATH="$manual_packages"
            typst compile --creation-timestamp 0 --root . symbolica/manual.typ symbolica/manual.pdf
          '');
          check = app "symbolica-check" (dependencyBoundaryScript + engineBuildScript + packageScript + ''
            check_dir="$(mktemp -d)"
            trap 'rm -rf "$check_dir"' EXIT
          '' + typstCheckScript);
          typst = app "symbolica-typst" ''exec typst "$@"'';
        });
      checks = eachSystem (pkgs: {
        default = pkgs.runCommand "symbolica-typst-check" {
          TYPST_FONT_PATHS = "${pkgs.dejavu_fonts}/share/fonts";
          TYPST_IGNORE_SYSTEM_FONTS = "true";
          nativeBuildInputs = [ pkgs.coreutils pkgs.diffutils pkgs.gnutar pkgs.gzip pkgs.python3 (typstWithPackages pkgs) ];
        } (''
          work="$TMPDIR/symbolica"
          mkdir -p "$work"
          cp -R ${self}/symbolica "$work/symbolica"
          cp -R ${self}/scripts "$work/scripts"
          for file in typst.toml README.md README-universe.md LICENSE LICENSE-SYMBOLICA.md LICENSE-SYMBOLICA-TYPST.md THIRD_PARTY.md THIRD_PARTY_LICENSES.txt REBUILDING.md CHANGELOG.md; do
            cp "${self}/$file" "$work/$file"
          done
          chmod -R u+w "$work"
          cd "$work"
          mkdir -p "$out"
          check_dir="$out"
        '' + packageScript + typstCheckScript);
      });
    };
}
