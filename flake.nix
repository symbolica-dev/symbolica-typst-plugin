{
  description = "Symbolica computer algebra and integration for Typst";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      typstWithPackages = pkgs: pkgs.typst.withPackages (packages: [
        packages.cetz_0_5_2 packages.cetz-plot_0_1_4
        packages.parsely_0_1_0 packages.tidy_0_4_3
      ]);
      typstCheckScript = ''
        for example in basic showcase expression-grid lotka-volterra phase-portrait integration; do
          typst compile --root . "symbolica/examples/$example.typ" "$check_dir/$example.pdf"
        done
        for fixture in api-surface parsely-metadata worked-examples integration; do
          typst compile --root . "symbolica/tests/$fixture.typ" "$check_dir/test-$fixture.pdf"
        done
        for manual in manual integration-manual; do
          typst compile --creation-timestamp 0 --root . "symbolica/$manual.typ" "$check_dir/$manual.pdf"
          if ! cmp -s "symbolica/$manual.pdf" "$check_dir/$manual.pdf"; then
            echo "symbolica/$manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
            exit 1
          fi
        done
        package_dir="$check_dir/packages/local/symbolica"
        mkdir -p "$package_dir"
        ln -s "$PWD" "$package_dir/0.1.0"
        for example in local-package integration-local-package; do
          typst compile --package-path "$check_dir/packages" --root . \
            "symbolica/examples/$example.typ" "$check_dir/$example.pdf"
        done
        rm -rf "$check_dir/packages"
      '';
      packageScript = ''
        mkdir -p dist
        # Match the runtime files kept by typst.toml's exclude list.
        tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf dist/symbolica-0.1.0.tar.gz \
          typst.toml README.md LICENSE THIRD_PARTY.md \
          symbolica/lib.typ symbolica/render.typ symbolica/symbolica.wasm
        size="$(wc -c < dist/symbolica-0.1.0.tar.gz)"
        if [ "$size" -gt 10485760 ]; then
          echo "Package archive is $size bytes; our download budget is 10 MiB" >&2
          exit 1
        fi
        ls -lh symbolica/symbolica.wasm dist/symbolica-0.1.0.tar.gz
      '';
    in {
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [ pkgs.binaryen pkgs.cargo pkgs.lld pkgs.rustc pkgs.rustfmt pkgs.stdenv.cc (typstWithPackages pkgs) ];
        };
      });
      apps = eachSystem (pkgs:
        let
          path = [ pkgs.binaryen pkgs.cargo pkgs.coreutils pkgs.diffutils pkgs.gnutar pkgs.gzip pkgs.lld pkgs.rustc pkgs.stdenv.cc (typstWithPackages pkgs) ];
          app = name: text: {
            type = "app";
            program = "${pkgs.writeShellApplication { inherit name; runtimeInputs = path; inherit text; }}/bin/${name}";
            meta.description = "Run ${name}";
          };
          engineBuildScript = ''

            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package symbolica-typst-plugin --no-default-features

            engine_raw="symbolica/symbolica.wasm"
            # Symbolica's C API exports keep unrelated code alive in the linker.
            # Keep simplify_expr, which belongs to this plugin's Typst API.
            wasm-opt --remove-exports --pass-arg='remove-exports@__getrandom*' \
              --remove-exports --pass-arg='remove-exports@drop' \
              --remove-exports --pass-arg='remove-exports@get_license_key' \
              --remove-exports --pass-arg='remove-exports@init' \
              --remove-exports --pass-arg='remove-exports@is_licensed' \
              --remove-exports --pass-arg='remove-exports@request_*' \
              --remove-exports --pass-arg='remove-exports@set_*' \
              --remove-exports --pass-arg='remove-exports@simplify' \
              --remove-exports --pass-arg='remove-exports@simplify_factorized' \
              --remove-exports --pass-arg='remove-exports@use_hu_*' \
              --remove-unused-module-elements -Oz --quiet \
              --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
              -o "$engine_raw" "target/$target/release/symbolica_typst_plugin.wasm"

          '';
          dependencyBoundaryScript = ''
            for package in symbolica-typst-plugin symbolica-typst-atom-payload; do
              tree="$(cargo tree --edges normal --prefix none --format '{p}' --package "$package")"
              for dependency in spenso idenso; do
                if [[ "$tree" == "$dependency v"* || "$tree" == *$'\n'"$dependency v"* ]]; then
                  echo "$package must not depend on $dependency" >&2
                  exit 1
                fi
              done
            done
            tree="$(cargo tree --edges normal --prefix none --format '{p}' --package symbolica-typst-atom-payload)"
            if [[ "$tree" == *"symbolica-integrate v"* ]]; then
              echo "The shared Atom payload crate must not depend on integration" >&2
              exit 1
            fi
          '';
        in rec {
          default = build;
          build = app "symbolica-build" engineBuildScript;
          build-engine = build;
          package = app "symbolica-package" (engineBuildScript + packageScript);
          manual = app "symbolica-manual" (engineBuildScript + ''
            for manual in manual integration-manual; do
              typst compile --creation-timestamp 0 --root . "symbolica/$manual.typ" "symbolica/$manual.pdf"
            done
          '');
          check = app "symbolica-check" (dependencyBoundaryScript + engineBuildScript + packageScript + ''
            check_dir="$(mktemp -d)"
            trap 'rm -rf "$check_dir"' EXIT
          '' + typstCheckScript);
          typst = app "symbolica-typst" ''exec typst "$@"'';
        });
      checks = eachSystem (pkgs: {
        default = pkgs.runCommand "symbolica-typst-check" {
          nativeBuildInputs = [ pkgs.coreutils pkgs.diffutils pkgs.gnutar pkgs.gzip (typstWithPackages pkgs) ];
        } (''
          work="$TMPDIR/symbolica"
          mkdir -p "$work"
          cp -R ${self}/symbolica "$work/symbolica"
          for file in typst.toml README.md LICENSE THIRD_PARTY.md; do
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
