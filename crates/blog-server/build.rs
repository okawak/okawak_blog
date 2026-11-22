use std::env::consts::{ARCH, OS};
use toml::Table;
use topcoat::tailwind::BuildConfig;

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");

    pinned_tailwind()
        .input("styles.css")
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}

fn pinned_tailwind() -> BuildConfig {
    const TOOL: &str = "aqua:tailwindlabs/tailwindcss";
    let config: Table = include_str!("../../mise.toml")
        .parse()
        .expect("valid mise.toml");
    let lock: Table = include_str!("../../mise.lock")
        .parse()
        .expect("valid mise.lock");
    let version = config["tools"][TOOL]
        .as_str()
        .expect("exact Tailwind version in mise.toml");

    let arch = match ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => panic!("unsupported Tailwind build architecture: {ARCH}"),
    };
    let platform = format!("platforms.{OS}-{arch}");
    let checksum = lock["tools"][TOOL]
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry["version"].as_str() == Some(version))
        })
        .and_then(|entry| entry.get(&platform)?.get("checksum")?.as_str())
        .expect("missing Tailwind checksum: run mise lock aqua:tailwindlabs/tailwindcss");

    BuildConfig::new().version_checksum(version, checksum)
}
