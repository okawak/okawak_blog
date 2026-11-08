#[path = "build_support/tailwind.rs"]
mod tailwind;

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");
    println!("cargo::rerun-if-changed=build_support");

    // `assets::STYLESHEET` registers the stylesheet rendered by this build integration.
    tailwind::retry_http(
        || {
            topcoat::tailwind::BuildConfig::new()
                .input("styles.css")
                .version("4.3.3")
                .render()
        },
        std::thread::sleep,
    )
    .expect("Topcoat should build the site Tailwind stylesheet");
}
