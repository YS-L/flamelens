fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let python_feature_enabled = std::env::var_os("CARGO_FEATURE_PYTHON").is_some();

    // On Linux, remoteprocess/unwind pulls libunwind-ptrace, which requires liblzma.
    // Explicitly linking lzma avoids unresolved lzma_* symbols under rust-lld.
    if target_os == "linux" && python_feature_enabled {
        println!("cargo:rustc-link-lib=lzma");
    }
}
