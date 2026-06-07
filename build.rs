fn main() {
    // Set the minimum macOS deployment target for the linked binary.
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");
}
