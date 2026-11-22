#[path = "build/tailwind.rs"]
mod tailwind;

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");

    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .executable(tailwind::verified_cli())
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}
