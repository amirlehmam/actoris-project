//! Build script for ACTORIS eBPF programs
//!
//! This script handles:
//! 1. Compiling eBPF programs for the BPF target
//! 2. Generating bindings
//! 3. Including compiled bytecode in the final binary

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Only build eBPF on Linux
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "linux" {
        println!("cargo:warning=eBPF programs only supported on Linux");
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Check if we're cross-compiling for eBPF
    let target = env::var("TARGET").unwrap();
    if target.contains("bpf") {
        // This is the eBPF target build, skip the build script
        return;
    }

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Build eBPF program using cargo
    let status = Command::new("cargo")
        .current_dir(&manifest_dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .args([
            "+nightly",
            "build",
            "-Z", "build-std=core",
            "--target", "bpfel-unknown-none",
            "--release",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=eBPF programs compiled successfully");
        }
        Ok(s) => {
            println!("cargo:warning=eBPF compilation failed with status: {}", s);
            // Don't fail the build - eBPF is optional
        }
        Err(e) => {
            println!("cargo:warning=Failed to run cargo for eBPF: {}", e);
            println!("cargo:warning=eBPF programs will not be available");
        }
    }

    // Copy the compiled eBPF object to OUT_DIR
    let ebpf_obj = manifest_dir
        .join("target")
        .join("bpfel-unknown-none")
        .join("release")
        .join("actoris-ebpf");

    if ebpf_obj.exists() {
        let dest = out_dir.join("actoris-ebpf.o");
        std::fs::copy(&ebpf_obj, &dest).ok();
        println!("cargo:rustc-env=ACTORIS_EBPF_OBJ={}", dest.display());
    } else {
        // Create a placeholder
        let placeholder = out_dir.join("actoris-ebpf.o");
        std::fs::write(&placeholder, b"").ok();
        println!("cargo:rustc-env=ACTORIS_EBPF_OBJ={}", placeholder.display());
    }
}
