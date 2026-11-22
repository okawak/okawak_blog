use std::env::consts::{ARCH, OS};

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");

    let host = format!("{OS}-{ARCH} v");
    let (version, checksum) = include_str!("tailwind.pins")
        .lines()
        .find_map(|line| line.strip_prefix(&host))
        .and_then(|pin| pin.split_once(' '))
        .expect("missing Tailwind release pin for this build host");

    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .version_checksum(version, checksum)
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}
