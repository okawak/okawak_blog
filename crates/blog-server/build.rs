use std::process::{Command, Stdio};

const TAILWIND: &str = "aqua:tailwindlabs/tailwindcss";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in ["src", "styles.css", "../../mise.toml", "../../mise.lock"] {
        println!("cargo::rerun-if-changed={path}");
    }

    let cli = Command::new("mise")
        .args(["which", "--locked", "--tool", TAILWIND, "tailwindcss"])
        .stderr(Stdio::inherit())
        .output()?;
    if !cli.status.success() {
        return Err("Install the locked Tailwind CLI with `mise install --locked`".into());
    }

    topcoat::tailwind::BuildConfig::new()
        .executable(std::str::from_utf8(&cli.stdout)?.trim_end())
        .input("styles.css")
        .render()?;
    Ok(())
}
