fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }

    #[cfg(feature = "system")]
    {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        if target_os == "macos" {
            // Tell Cargo to add `-framework Accelerate` to the linker command line.
            println!("cargo:rustc-link-lib=framework=Accelerate");
        }
    }
}
