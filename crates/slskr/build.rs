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
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(repo_root) = manifest_dir.parent().and_then(|path| path.parent()) else {
        eprintln!(
            "Unable to resolve repository root from {}",
            manifest_dir.display()
        );
        return;
    };
    let web_root = repo_root.join("web");

    // The workspace carries a small local mainline extension for preserving
    // the public source port of shared DHT traffic. Cargo package archives
    // resolve the published mainline API instead, so expose a compile-time
    // capability only when that workspace source is actually present.
    println!("cargo:rustc-check-cfg=cfg(slskr_mainline_outbound_socket)");
    let mainline_extension = repo_root.join("vendor/mainline/src/dht.rs");
    if mainline_extension.exists() {
        println!("cargo:rustc-cfg=slskr_mainline_outbound_socket");
        println!("cargo:rerun-if-changed={}", mainline_extension.display());
    }

    if web_root.exists() {
        // Rebuild if any src file changes
        println!("cargo:rerun-if-changed={}/src", web_root.display());
        println!("cargo:rerun-if-changed={}/public", web_root.display());
        println!("cargo:rerun-if-changed={}/index.html", web_root.display());
        println!("cargo:rerun-if-changed={}/package.json", web_root.display());
    }

    // Optionally build webui if requested via env var
    if std::env::var("SLSKR_BUILD_WEB").is_ok() && web_root.exists() {
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
                    "Webui build failed with status {}.\nstdout:\n{}\nstderr:\n{}",
                    out.status,
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr),
                );
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to run npm build: {}", e);
                std::process::exit(1);
            }
        }
    }
}
