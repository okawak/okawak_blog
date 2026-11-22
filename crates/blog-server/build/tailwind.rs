use sha2::{Digest, Sha256};
use std::{env, fs::File, io::Read, path::PathBuf};
use topcoat::tailwind::ExecutableSource;

const VERSION: &str = "4.3.3";

// Release asset SHA-256 values from tailwindlabs/tailwindcss v4.3.3.
fn host_asset() -> (&'static str, &'static str) {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => (
            "linux-x64",
            "dc61b3ac6b8c9ca874c0cc4c57b2409791a64c5540404ca5f5367360babc313a",
        ),
        ("linux", "aarch64") => (
            "linux-arm64",
            "55fd0b241214eff3de1e8ee4f22796662f2d2e7a49bcfca7477cfd0bac398195",
        ),
        ("macos", "x86_64") => (
            "macos-x64",
            "7922e0953f2110c05976e3bf58f14e643d90427575e766b7d433f5f80cbee7e1",
        ),
        ("macos", "aarch64") => (
            "macos-arm64",
            "cdf646702987a743464dff4d9c60fd4480d1c1e73dd819a9a67f1078815dce9d",
        ),
        ("windows", "x86_64") => (
            "windows-x64.exe",
            "e0e260ce048014e9268f6237ff18f8ccf02cef521cbd0ae04e82c2cdf7aa3955",
        ),
        host => panic!("no pinned Tailwind asset for {host:?}"),
    }
}

pub(super) fn verified_cli() -> PathBuf {
    let (platform, expected) = host_asset();
    ExecutableSource::Github {
        version: VERSION.into(),
        checksum: Some(format!("sha256:{expected}")),
    }
    .resolve()
    .expect("download the pinned Tailwind binary");

    // Topcoat verifies downloads but reuses cached executables without checking them.
    let cache = topcoat_core::cache::cache_dir("tailwind")
        .unwrap_or_else(|| PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR")));
    let cli = cache.join(format!("tailwindcss-{VERSION}-{platform}"));
    let binary = File::open(&cli).expect("open Tailwind binary");
    assert_eq!(
        sha256(binary),
        expected,
        "Tailwind binary checksum mismatch"
    );
    println!("cargo::rerun-if-changed={}", cli.display());
    cli
}

fn sha256(mut binary: File) -> String {
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = binary.read(&mut buffer).expect("hash Tailwind binary");
        if n == 0 {
            return hex::encode(digest.finalize());
        }
        digest.update(&buffer[..n]);
    }
}
