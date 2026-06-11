fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");

    vergen_git2::Emitter::default()
        .add_instructions(
            &vergen_git2::Git2Builder::default()
                .describe(true, true, None)
                .build()
                .unwrap(),
        )
        .unwrap()
        .emit()
        .unwrap();
}
