use std::{env, fs, path::PathBuf};

const REVISION_PREFIX: &str = "rev = \"";

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must set CARGO_MANIFEST_DIR"),
    );
    let adapter_manifest = manifest_dir.join("../../vendor/symbolica-wasm/Cargo.toml");
    println!("cargo:rerun-if-changed={}", adapter_manifest.display());

    let manifest = fs::read_to_string(&adapter_manifest).unwrap_or_else(|error| {
        panic!(
            "could not read Symbolica adapter manifest {}: {error}",
            adapter_manifest.display()
        )
    });
    let dependency = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("symbolica-upstream ="))
        .expect("Symbolica adapter manifest has no symbolica-upstream dependency");
    let revision_start = dependency
        .find(REVISION_PREFIX)
        .map(|index| index + REVISION_PREFIX.len())
        .expect("symbolica-upstream dependency has no pinned revision");
    let revision = dependency[revision_start..]
        .split_once('"')
        .map(|(revision, _)| revision)
        .expect("symbolica-upstream revision is not quoted");

    assert!(
        revision.len() == 40
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "symbolica-upstream revision must be a 40-character lowercase hexadecimal Git ID"
    );
    println!("cargo:rustc-env=TYMBOLICA_SYMBOLICA_REVISION={revision}");
}
