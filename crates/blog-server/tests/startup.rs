#[test]
fn server_rejects_invalid_artifact_configuration_before_starting() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_blog-server"))
        .env("OKAWAK_BLOG_ARTIFACT_SOURCE", "unsupported-test-source")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported-test-source"));
}
