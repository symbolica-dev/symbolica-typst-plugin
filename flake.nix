{
  description = "Symbolica computer algebra for Typst";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      typstWithPackages = pkgs: pkgs.typst.withPackages (packages: [
        packages.cetz_0_5_2
        packages.cetz-plot_0_1_4
        packages.parsely_0_1_0
        packages.tidy_0_4_3
      ]);
    in {
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [ pkgs.binaryen pkgs.cargo pkgs.lld pkgs.rustc pkgs.rustfmt pkgs.wizer (typstWithPackages pkgs) ];
        };
      });

      apps = eachSystem (pkgs:
        let
          path = [ pkgs.binaryen pkgs.cargo pkgs.coreutils pkgs.diffutils pkgs.lld pkgs.rustc pkgs.wizer (typstWithPackages pkgs) ];
          app = name: text: {
            type = "app";
            program = "${pkgs.writeShellApplication { inherit name; runtimeInputs = path; inherit text; }}/bin/${name}";
            meta.description = "Run ${name}";
          };
          loaderBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tymbolica-inflate-plugin --lib
            for output in typst/tymbolica-inflate.wasm tydenso/tydenso-inflate.wasm rubi/tymbolica-inflate.wasm; do
              wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
                -o "$output" "target/$target/release/tymbolica_inflate_plugin.wasm"
              size="$(wc -c < "$output")"
              if [ "$size" -gt 10485760 ]; then
                echo "$output is $size bytes; the loader must remain below 10 MiB" >&2
                exit 1
              fi
              ls -lh "$output"
            done
          '';
          engineBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tymbolica-plugin --no-default-features

            engine_raw="target/$target/release/tymbolica.raw.wasm"
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
              -o "$engine_raw" "target/$target/release/tymbolica_plugin.wasm"

            cargo run --release --package tymbolica-inflate-plugin --bin tymbolica-compress -- \
              "$engine_raw" typst/tymbolica.wasm.zlib
            size="$(wc -c < typst/tymbolica.wasm.zlib)"
            if [ "$size" -gt 10485760 ]; then
              echo "typst/tymbolica.wasm.zlib is $size bytes; the compressed engine must remain below 10 MiB" >&2
              exit 1
            fi
            ls -lh typst/tymbolica.wasm.zlib
          '';
          tydensoBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tydenso-plugin
            tydenso_raw="target/$target/release/tydenso.raw.wasm"
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
              -o "$tydenso_raw" "target/$target/release/tydenso_plugin.wasm"

            cargo run --release --package tymbolica-inflate-plugin --bin tymbolica-compress -- \
              "$tydenso_raw" tydenso/tydenso.wasm.zlib
            size="$(wc -c < tydenso/tydenso.wasm.zlib)"
            if [ "$size" -gt 10485760 ]; then
              echo "tydenso/tydenso.wasm.zlib is $size bytes; the compressed Tydenso engine must remain below 10 MiB" >&2
              exit 1
            fi
            ls -lh tydenso/tydenso.wasm.zlib
          '';
          rubiBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tymbolica-rubi-plugin \
              --no-default-features --features wizer-preinitialize

            rubi_preinitialized_input="target/$target/release/tymbolica-rubi.preinitialize.wasm"
            rubi_wizened="target/$target/release/tymbolica-rubi.wizened.wasm"
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd \
              --remove-exports --pass-arg='remove-exports@__getrandom*' \
              --remove-exports --pass-arg='remove-exports@drop' \
              --remove-exports --pass-arg='remove-exports@get_license_key' \
              --remove-exports --pass-arg='remove-exports@init' \
              --remove-exports --pass-arg='remove-exports@is_licensed' \
              --remove-exports --pass-arg='remove-exports@request_*' \
              --remove-exports --pass-arg='remove-exports@set_*' \
              --remove-exports --pass-arg='remove-exports@simplify*' \
              --remove-exports --pass-arg='remove-exports@use_hu_*' \
              --remove-unused-module-elements --strip-debug --strip-producers \
              -o "$rubi_preinitialized_input" "target/$target/release/tymbolica_rubi_plugin.wasm"
            wizer --init-func wizer.initialize "$rubi_preinitialized_input" -o "$rubi_wizened"

            cargo run --release --package tymbolica-inflate-plugin --bin tymbolica-compress -- \
              "$rubi_wizened" rubi/tymbolica-rubi.wasm.zlib
            size="$(wc -c < rubi/tymbolica-rubi.wasm.zlib)"
            if [ "$size" -gt 10485760 ]; then
              echo "rubi/tymbolica-rubi.wasm.zlib is $size bytes; the compressed Rubi engine must remain below 10 MiB" >&2
              exit 1
            fi
            ls -lh rubi/tymbolica-rubi.wasm.zlib
          '';
          dependencyBoundaryScript = ''
            assert_dependency_absent() {
              local package="$1"
              local dependency="$2"
              local tree
              tree="$(cargo tree --edges normal --prefix none --format '{p}' --package "$package")"
              if [[ "$tree" == "$dependency v"* || "$tree" == *$'\n'"$dependency v"* ]]; then
                echo "$package must not depend on $dependency" >&2
                exit 1
              fi
            }

            for dependency in spenso idenso symbolica-integrate; do
              assert_dependency_absent tymbolica-plugin "$dependency"
              assert_dependency_absent tymbolica-typst-ast "$dependency"
            done
            for dependency in spenso idenso; do
              assert_dependency_absent tymbolica-rubi-plugin "$dependency"
            done
          '';
          buildScript = loaderBuildScript + engineBuildScript + tydensoBuildScript + rubiBuildScript;
        in rec {
          default = build;
          build = app "tymbolica-build" buildScript;
          build-engine = app "tymbolica-build-engine" (loaderBuildScript + engineBuildScript);
          build-tydenso = app "tymbolica-build-tydenso" (loaderBuildScript + tydensoBuildScript);
          build-rubi = app "tymbolica-build-rubi" (loaderBuildScript + rubiBuildScript);
          manual = app "tymbolica-manual" (buildScript + ''
            tymbolica_out="''${TYMBOLICA_MANUAL_OUT:-typst/manual.pdf}"
            tydenso_out="''${TYDENSO_MANUAL_OUT:-tydenso/manual.pdf}"
            rubi_out="''${TYMBOLICA_RUBI_MANUAL_OUT:-rubi/manual.pdf}"
            mkdir -p "$(dirname "$tymbolica_out")" "$(dirname "$tydenso_out")" "$(dirname "$rubi_out")"
            typst compile --root . typst/manual.typ "$tymbolica_out"
            typst compile --root . tydenso/manual.typ "$tydenso_out"
            typst compile --root . rubi/manual.typ "$rubi_out"
            ls -lh "$tymbolica_out" "$tydenso_out" "$rubi_out"
          '');
          check = app "tymbolica-check" (dependencyBoundaryScript + buildScript + ''
            check_dir="$(mktemp -d)"
            trap 'rm -rf "$check_dir"' EXIT

            typst compile --root . typst/examples/basic.typ "$check_dir/basic.pdf"
            typst compile --root . typst/examples/showcase.typ "$check_dir/showcase.pdf"
            typst compile --root . typst/examples/expression-grid.typ "$check_dir/expression-grid.pdf"
            typst compile --root . typst/examples/lotka-volterra.typ "$check_dir/lotka-volterra.pdf"
            typst compile --root . typst/examples/phase-portrait.typ "$check_dir/phase-portrait.pdf"
            typst compile --root . typst/examples/api-surface.typ "$check_dir/api-surface.pdf"
            typst compile --root . typst/examples/parsely-mwe.typ "$check_dir/parsely-mwe.pdf"
            typst compile --root . typst/manual.typ "$check_dir/manual.pdf"
            typst compile --root . tydenso/examples/basic.typ "$check_dir/tydenso-basic.pdf"
            typst compile --root . tydenso/examples/symmetry.typ "$check_dir/tydenso-symmetry.pdf"
            typst compile --root . tydenso/examples/interop.typ "$check_dir/tydenso-interop.pdf"
            typst compile --root . tydenso/examples/spenso-notation.typ "$check_dir/tydenso-spenso-notation.pdf"
            typst compile --root . tydenso/examples/index-palettes.typ "$check_dir/tydenso-index-palettes.pdf"
            typst compile --root . tydenso/manual.typ "$check_dir/tydenso-manual.pdf"
            typst compile --root . rubi/examples/basic.typ "$check_dir/rubi-basic.pdf"
            typst compile --root . rubi/manual.typ "$check_dir/rubi-manual.pdf"

            package_dir="$check_dir/xdg/typst/packages/local/tymbolica"
            mkdir -p "$package_dir"
            ln -s "$PWD" "$package_dir/0.1.0"
            XDG_DATA_HOME="$check_dir/xdg" \
              typst compile --root . typst/examples/local-package.typ "$check_dir/local-package.pdf"

            tydenso_package_dir="$check_dir/xdg/typst/packages/local/tydenso"
            mkdir -p "$tydenso_package_dir"
            ln -s "$PWD/tydenso" "$tydenso_package_dir/0.1.0"
            XDG_DATA_HOME="$check_dir/xdg" \
              typst compile --root . tydenso/examples/local-package.typ "$check_dir/tydenso-local-package.pdf"

            rubi_package_dir="$check_dir/xdg/typst/packages/local/tymbolica-rubi"
            mkdir -p "$rubi_package_dir"
            ln -s "$PWD/rubi" "$rubi_package_dir/0.1.0"
            XDG_DATA_HOME="$check_dir/xdg" \
              typst compile --root . rubi/examples/local-package.typ "$check_dir/rubi-local-package.pdf"

            if ! cmp -s typst/manual.pdf "$check_dir/manual.pdf"; then
              echo "typst/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
            if ! cmp -s tydenso/manual.pdf "$check_dir/tydenso-manual.pdf"; then
              echo "tydenso/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
            if ! cmp -s rubi/manual.pdf "$check_dir/rubi-manual.pdf"; then
              echo "rubi/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
          '');
          typst = app "tymbolica-typst" ''exec typst "$@"'';
        });

      checks = eachSystem (pkgs:
        let
          typst = typstWithPackages pkgs;
        in {
          default = pkgs.runCommand "tymbolica-typst-check" {
            nativeBuildInputs = [ pkgs.coreutils pkgs.diffutils pkgs.findutils typst ];
          } ''
            work="$TMPDIR/tymbolica"
            mkdir -p "$work"
            cp -R ${self}/typst "$work/typst"
            cp -R ${self}/tydenso "$work/tydenso"
            cp -R ${self}/rubi "$work/rubi"
            cp ${self}/typst.toml "$work/typst.toml"
            chmod -R u+w "$work"
            mkdir -p "$out"

            while IFS= read -r -d "" file; do
              size="$(wc -c < "$file")"
              if [ "$size" -gt 20971520 ]; then
                echo "$file is $size bytes; Typst web app files must not exceed 20 MiB" >&2
                exit 1
              fi
            done < <(find "$work/typst" "$work/tydenso" "$work/rubi" -type f -print0)

            while IFS= read -r -d "" file; do
              size="$(wc -c < "$file")"
              if [ "$size" -gt 10485760 ]; then
                echo "$file is $size bytes; compressed plugin assets must remain below 10 MiB" >&2
                exit 1
              fi
            done < <(find "$work/typst" "$work/tydenso" "$work/rubi" -type f -name '*.wasm.zlib' -print0)

            typst compile --root "$work" "$work/typst/examples/basic.typ" "$out/basic.pdf"
            typst compile --root "$work" "$work/typst/examples/showcase.typ" "$out/showcase.pdf"
            typst compile --root "$work" "$work/typst/examples/expression-grid.typ" "$out/expression-grid.pdf"
            typst compile --root "$work" "$work/typst/examples/lotka-volterra.typ" "$out/lotka-volterra.pdf"
            typst compile --root "$work" "$work/typst/examples/phase-portrait.typ" "$out/phase-portrait.pdf"
            typst compile --root "$work" "$work/typst/examples/api-surface.typ" "$out/api-surface.pdf"
            typst compile --root "$work" "$work/typst/examples/parsely-mwe.typ" "$out/parsely-mwe.pdf"
            typst compile --root "$work" "$work/typst/manual.typ" "$out/manual.pdf"
            typst compile --root "$work" "$work/tydenso/examples/basic.typ" "$out/tydenso-basic.pdf"
            typst compile --root "$work" "$work/tydenso/examples/symmetry.typ" "$out/tydenso-symmetry.pdf"
            typst compile --root "$work" "$work/tydenso/examples/interop.typ" "$out/tydenso-interop.pdf"
            typst compile --root "$work" "$work/tydenso/examples/spenso-notation.typ" "$out/tydenso-spenso-notation.pdf"
            typst compile --root "$work" "$work/tydenso/examples/index-palettes.typ" "$out/tydenso-index-palettes.pdf"
            typst compile --root "$work" "$work/tydenso/manual.typ" "$out/tydenso-manual.pdf"
            typst compile --root "$work" "$work/rubi/examples/basic.typ" "$out/rubi-basic.pdf"
            typst compile --root "$work" "$work/rubi/manual.typ" "$out/rubi-manual.pdf"

            package_dir="$TMPDIR/xdg/typst/packages/local/tymbolica"
            mkdir -p "$package_dir"
            ln -s "$work" "$package_dir/0.1.0"
            XDG_DATA_HOME="$TMPDIR/xdg" \
              typst compile --root "$work" "$work/typst/examples/local-package.typ" "$out/local-package.pdf"

            tydenso_package_dir="$TMPDIR/xdg/typst/packages/local/tydenso"
            mkdir -p "$tydenso_package_dir"
            ln -s "$work/tydenso" "$tydenso_package_dir/0.1.0"
            XDG_DATA_HOME="$TMPDIR/xdg" \
              typst compile --root "$work" "$work/tydenso/examples/local-package.typ" "$out/tydenso-local-package.pdf"

            rubi_package_dir="$TMPDIR/xdg/typst/packages/local/tymbolica-rubi"
            mkdir -p "$rubi_package_dir"
            ln -s "$work/rubi" "$rubi_package_dir/0.1.0"
            XDG_DATA_HOME="$TMPDIR/xdg" \
              typst compile --root "$work" "$work/rubi/examples/local-package.typ" "$out/rubi-local-package.pdf"

            if ! cmp -s "$work/typst/manual.pdf" "$out/manual.pdf"; then
              echo "typst/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
            if ! cmp -s "$work/tydenso/manual.pdf" "$out/tydenso-manual.pdf"; then
              echo "tydenso/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
            if ! cmp -s "$work/rubi/manual.pdf" "$out/rubi-manual.pdf"; then
              echo "rubi/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
          '';
        });
    };
}
