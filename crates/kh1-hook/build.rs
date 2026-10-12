fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows-gnu") {
        println!("cargo:rustc-cdylib-link-arg=-static-libgcc");
    }
    // A build stamp for the log, so a test run shows which DLL it used.
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    println!("cargo:rustc-env=KH1_KH2_BUILD={secs}");
}
