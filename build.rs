fn main() {
    // Su macOS, tray-icon richiede che l'app abbia un bundle identifier
    // Questo viene letto a runtime via Info.plist oppure impostato così:
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");
}
