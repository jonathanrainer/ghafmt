fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let metadata = std::env::var("GHAFMT_BUILD_METADATA").unwrap_or_default();
    let version_string = if metadata.is_empty() {
        version
    } else {
        format!("{version}-{metadata}")
    };
    println!("cargo:rustc-env=GHAFMT_VERSION_STRING={version_string}");
    println!("cargo:rerun-if-env-changed=GHAFMT_BUILD_METADATA");
}
