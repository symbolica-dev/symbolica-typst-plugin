{
  description = "Symbolica computer algebra for Typst";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      typstWithPackages = pkgs: pkgs.typst.withPackages (packages: [
        packages.parsely_0_1_0
        packages.tidy_0_4_3
      ]);
    in {
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [ pkgs.binaryen pkgs.cargo pkgs.lld pkgs.rustc (typstWithPackages pkgs) ];
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
          buildScript = ''
            target=wasm32-unknown-unknown
            unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
            cargo build --release --target "$target"
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --strip-debug --strip-producers \
              -o typst/tymbolica.wasm "target/$target/release/tymbolica_plugin.wasm"
            ls -lh typst/tymbolica.wasm
          '';
        in rec {
          default = build;
          build = app "tymbolica-build" buildScript;
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
            typst compile --root . typst/examples/api-surface.typ "$check_dir/api-surface.pdf"
            typst compile --root . typst/examples/parsely-mwe.typ "$check_dir/parsely-mwe.pdf"
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
            nativeBuildInputs = [ pkgs.coreutils pkgs.diffutils typst ];
          } ''
            work="$TMPDIR/tymbolica"
            mkdir -p "$work"
            cp -R ${self}/typst "$work/typst"
            cp ${self}/typst.toml "$work/typst.toml"
            chmod -R u+w "$work"
            mkdir -p "$out"

            typst compile --root "$work" "$work/typst/examples/basic.typ" "$out/basic.pdf"
            typst compile --root "$work" "$work/typst/examples/showcase.typ" "$out/showcase.pdf"
            typst compile --root "$work" "$work/typst/examples/api-surface.typ" "$out/api-surface.pdf"
            typst compile --root "$work" "$work/typst/examples/parsely-mwe.typ" "$out/parsely-mwe.pdf"
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
