fn main() {
    for path in ["src", "styles.css", "../../mise.toml", "../../mise.lock"] {
        println!("cargo::rerun-if-changed={path}");
    }

    topcoat::tailwind::BuildConfig::new()
        .executable("tailwindcss")
        .input("styles.css")
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}
