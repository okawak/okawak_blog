fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=styles.css");

    // `assets::STYLESHEET` registers the stylesheet rendered by this build integration.
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .version("4.3.3")
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}
