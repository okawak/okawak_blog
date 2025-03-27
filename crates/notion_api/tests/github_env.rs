use notion_api::load_config;

#[test]
#[ignore]
fn test_github_env() {
    let config = load_config();
    assert!(config.is_ok());
}
