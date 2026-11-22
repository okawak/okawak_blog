use std::{env, path::PathBuf};
use topcoat::tailwind::{BuildConfig, ExecutableSource};

const VERSION: &str = "4.3.3";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");

    let host = format!("{}-{} ", env::consts::OS, env::consts::ARCH);
    let (platform, checksum) = include_str!("tailwind.pins")
        .lines()
        .find_map(|line| line.strip_prefix(&host))
        .and_then(|pin| pin.split_once(' '))
        .unwrap_or_else(|| panic!("missing Tailwind pin for {}", host.trim_end()));

    // Resolve before checking: another build may fill an initially empty cache.
    ExecutableSource::Github {
        version: VERSION.into(),
        checksum: Some(format!("sha256:{checksum}")),
    }
    .resolve()?;

    // Topcoat 0.12 verifies downloads, but trusts binaries already in its cache.
    let cache = topcoat_core::cache::cache_dir("tailwind")
        .unwrap_or_else(|| PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR")));
    let cli = cache.join(format!("tailwindcss-{VERSION}-{platform}"));
    println!("cargo::rerun-if-changed={}", cli.display());
    let actual = sha256::try_digest(&cli)?;
    assert_eq!(actual, checksum, "Tailwind binary checksum mismatch");

    BuildConfig::new()
        .input("styles.css")
        .executable(cli)
        .render()?;
    Ok(())
}
