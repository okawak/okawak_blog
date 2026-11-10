#[test]
fn server_rejects_invalid_artifact_configuration_before_starting() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_blog-server"))
        .env_remove("OKAWAK_BLOG_SITE_ORIGIN")
        .env("OKAWAK_BLOG_ARTIFACT_SOURCE", "unsupported-test-source")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported-test-source"));
}

#[test]
fn server_rejects_invalid_site_origin_before_loading_artifacts() {
    for origin in [
        "relative/path",
        "ftp://example.com",
        "https://example.com/?q=1",
        "https://example.com/#fragment",
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_blog-server"))
            .env("OKAWAK_BLOG_SITE_ORIGIN", origin)
            .env("OKAWAK_BLOG_ARTIFACT_SOURCE", "unsupported-test-source")
            .output()
            .unwrap();
        assert!(!output.status.success(), "{origin}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("OKAWAK_BLOG_SITE_ORIGIN"),
            "{origin}: {error}"
        );
        assert!(
            !error.contains("unsupported-test-source"),
            "{origin}: {error}"
        );
    }
}
