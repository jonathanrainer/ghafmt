fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let display = version.split_once('-').map_or(version.as_str(), |(_, pre)| pre);
    println!("cargo:rustc-env=GHAFMT_VERSION_STRING={display}");
}
