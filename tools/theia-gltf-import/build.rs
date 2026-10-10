fn main() {
    let compiler = std::env::var("RUSTC").expect("Cargo provides RUSTC");
    let output = std::process::Command::new(compiler)
        .arg("--version")
        .output()
        .expect("read compiler version");
    assert!(output.status.success());
    println!(
        "cargo:rustc-env=THEIA_IMPORT_RUSTC={}",
        String::from_utf8(output.stdout).unwrap().trim()
    );
    println!("cargo:rerun-if-changed=build.rs");
}
