fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=style");

    topcoat::tailwind::BuildConfig::new()
        .input("crates/site/web/style/tailwind.css")
        .cwd("../../..")
        .version("4.3.3")
        .render()
        .expect("Topcoat should build the site Tailwind stylesheet");
}
