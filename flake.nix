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
          packages = [ pkgs.binaryen pkgs.cargo pkgs.lld pkgs.rustc pkgs.rustfmt (typstWithPackages pkgs) ];
        };
      });

      apps = eachSystem (pkgs:
        let
          path = [ pkgs.binaryen pkgs.cargo pkgs.coreutils pkgs.diffutils pkgs.lld pkgs.rustc (typstWithPackages pkgs) ];
          app = name: text: {
            type = "app";
            program = "${pkgs.writeShellApplication { inherit name; runtimeInputs = path; inherit text; }}/bin/${name}";
            meta.description = "Run ${name}";
          };
          loaderBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tymbolica-inflate-plugin --lib
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
              -o typst/tymbolica-inflate.wasm "target/$target/release/tymbolica_inflate_plugin.wasm"
            size="$(wc -c < typst/tymbolica-inflate.wasm)"
            if [ "$size" -gt 10485760 ]; then
              echo "typst/tymbolica-inflate.wasm is $size bytes; the loader must remain below 10 MiB" >&2
              exit 1
            fi
            ls -lh typst/tymbolica-inflate.wasm
          '';
          engineBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tymbolica-plugin --no-default-features --features compressed-step-metadata

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
          idensoBuildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target" --package tymbolica-idenso-plugin
            idenso_raw="target/$target/release/tymbolica-idenso.raw.wasm"
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
              -o "$idenso_raw" "target/$target/release/tymbolica_idenso_plugin.wasm"

            cargo run --release --package tymbolica-inflate-plugin --bin tymbolica-compress -- \
              "$idenso_raw" typst/tymbolica-idenso.wasm.zlib
            size="$(wc -c < typst/tymbolica-idenso.wasm.zlib)"
            if [ "$size" -gt 10485760 ]; then
              echo "typst/tymbolica-idenso.wasm.zlib is $size bytes; the compressed Idenso engine must remain below 10 MiB" >&2
              exit 1
            fi
            ls -lh typst/tymbolica-idenso.wasm.zlib
          '';
          buildScript = loaderBuildScript + engineBuildScript + idensoBuildScript;
        in rec {
          default = build;
          build = app "tymbolica-build" buildScript;
          build-engine = app "tymbolica-build-engine" (loaderBuildScript + engineBuildScript);
          build-idenso = app "tymbolica-build-idenso" (loaderBuildScript + idensoBuildScript);
          manual = app "tymbolica-manual" (buildScript + ''
            out="''${TYMBOLICA_MANUAL_OUT:-typst/manual.pdf}"
            if [ "$#" -gt 0 ]; then
              out="$1"
              shift
            fi
            mkdir -p "$(dirname "$out")"
            typst compile --root . typst/manual.typ "$out" "$@"
            ls -lh "$out"
          '');
          check = app "tymbolica-check" (buildScript + ''
            check_dir="$(mktemp -d)"
            trap 'rm -rf "$check_dir"' EXIT

            typst compile --root . typst/examples/basic.typ "$check_dir/basic.pdf"
            typst compile --root . typst/examples/showcase.typ "$check_dir/showcase.pdf"
            typst compile --root . typst/examples/expression-grid.typ "$check_dir/expression-grid.pdf"
            typst compile --root . typst/examples/lotka-volterra.typ "$check_dir/lotka-volterra.pdf"
            typst compile --root . typst/examples/phase-portrait.typ "$check_dir/phase-portrait.pdf"
            typst compile --root . typst/examples/api-surface.typ "$check_dir/api-surface.pdf"
            typst compile --root . typst/examples/integration.typ "$check_dir/integration.pdf"
            typst compile --root . typst/examples/parsely-mwe.typ "$check_dir/parsely-mwe.pdf"
            typst compile --root . typst/examples/idenso.typ "$check_dir/idenso.pdf"
            typst compile --root . typst/manual.typ "$check_dir/manual.pdf"

            package_dir="$check_dir/xdg/typst/packages/local/tymbolica"
            mkdir -p "$package_dir"
            ln -s "$PWD" "$package_dir/0.1.0"
            XDG_DATA_HOME="$check_dir/xdg" \
              typst compile --root . typst/examples/local-package.typ "$check_dir/local-package.pdf"

            if ! cmp -s typst/manual.pdf "$check_dir/manual.pdf"; then
              echo "typst/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
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
            cp ${self}/typst.toml "$work/typst.toml"
            chmod -R u+w "$work"
            mkdir -p "$out"

            while IFS= read -r -d "" file; do
              size="$(wc -c < "$file")"
              if [ "$size" -gt 20971520 ]; then
                echo "$file is $size bytes; Typst web app files must not exceed 20 MiB" >&2
                exit 1
              fi
            done < <(find "$work/typst" -type f -print0)

            typst compile --root "$work" "$work/typst/examples/basic.typ" "$out/basic.pdf"
            typst compile --root "$work" "$work/typst/examples/showcase.typ" "$out/showcase.pdf"
            typst compile --root "$work" "$work/typst/examples/expression-grid.typ" "$out/expression-grid.pdf"
            typst compile --root "$work" "$work/typst/examples/lotka-volterra.typ" "$out/lotka-volterra.pdf"
            typst compile --root "$work" "$work/typst/examples/phase-portrait.typ" "$out/phase-portrait.pdf"
            typst compile --root "$work" "$work/typst/examples/api-surface.typ" "$out/api-surface.pdf"
            typst compile --root "$work" "$work/typst/examples/integration.typ" "$out/integration.pdf"
            typst compile --root "$work" "$work/typst/examples/parsely-mwe.typ" "$out/parsely-mwe.pdf"
            typst compile --root "$work" "$work/typst/examples/idenso.typ" "$out/idenso.pdf"
            typst compile --root "$work" "$work/typst/manual.typ" "$out/manual.pdf"

            package_dir="$TMPDIR/xdg/typst/packages/local/tymbolica"
            mkdir -p "$package_dir"
            ln -s "$work" "$package_dir/0.1.0"
            XDG_DATA_HOME="$TMPDIR/xdg" \
              typst compile --root "$work" "$work/typst/examples/local-package.typ" "$out/local-package.pdf"

            if ! cmp -s "$work/typst/manual.pdf" "$out/manual.pdf"; then
              echo "typst/manual.pdf is stale; run 'nix run .#manual' and commit it" >&2
              exit 1
            fi
          '';
        });
    };
}
