use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    if let Some(rustc) = rustc_version() {
        if rustc.minor < 60 {
            println!("cargo:warning=The cxx crate requires a rustc version 1.60.0 or newer.");
            println!(
                "cargo:warning=You appear to be building with: {}",
                rustc.version,
            );
        }
    }
}

struct RustVersion {
    version: String,
    minor: u32,
}

fn rustc_version() -> Option<RustVersion> {
    let rustc = env::var_os("RUSTC")?;
    let output = Command::new(rustc).arg("--version").output().ok()?;
    let version = String::from_utf8(output.stdout).ok()?;
    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }
    let minor = pieces.next()?.parse().ok()?;
    Some(RustVersion { version, minor })
}
