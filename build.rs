fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32")
        && std::env::var_os("CARGO_FEATURE_RUBI").is_some()
    {
        // Rubi's generated matcher comes close to wasm-ld's 1 MiB default
        // while constructing and traversing the integration rule table.
        println!("cargo:rustc-link-arg=-zstack-size=8388608");
    }
}
