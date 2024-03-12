use std::{env, process::Command};

fn main() {
    let rustc = env::var("RUSTC").unwrap();

    let output = Command::new(rustc).arg("-vV").output().unwrap();

    let version_info = String::from_utf8_lossy(&output.stdout);


    if let Some(major) = version_info
        .lines()
        .find_map(|x| x.strip_prefix("LLVM version: "))
        .and_then(|x| x.split('.').next())
    {
        println!("cargo:rustc-cfg=llvm_{}", major);
    }
}
