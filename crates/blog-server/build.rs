use std::env::consts::{ARCH, OS};

const VERSION: &str = "4.3.3";

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");

    let host = format!("{OS}-{ARCH} ");
    let checksum = include_str!("tailwind.pins")
        .lines()
        .find_map(|pin| pin.strip_prefix(&host))
        .expect("missing Tailwind checksum for this build host");

    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .version_checksum(VERSION, checksum)
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}
