//! Build script for slskr
//!
//! This script:
//! - Tracks changes to webui source files so cargo knows when to rebuild
//! - Runs webui build if SLSKR_BUILD_WEB env var is set
//! - Embeds webui dist files as static assets

use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Track webui source changes
    let web_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("web");

    if web_root.exists() {
        // Rebuild if any src file changes
        println!("cargo:rerun-if-changed={}/src", web_root.display());
        println!("cargo:rerun-if-changed={}/public", web_root.display());
        println!("cargo:rerun-if-changed={}/index.html", web_root.display());
        println!("cargo:rerun-if-changed={}/package.json", web_root.display());
    }

    // Optionally build webui if requested via env var
    if std::env::var("SLSKR_BUILD_WEB").is_ok() {
        if web_root.exists() {
            println!("Building webui...");
            let output = Command::new("npm")
                .arg("--prefix")
                .arg(&web_root)
                .arg("run")
                .arg("build")
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    println!("Webui built successfully");
                }
                Ok(out) => {
                    eprintln!(
                        "Webui build failed:\n{}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                Err(e) => {
                    eprintln!("Failed to run npm build: {}", e);
                }
            }
        }
    }
}
