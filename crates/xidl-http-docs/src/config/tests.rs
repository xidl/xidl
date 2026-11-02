use super::DocumentationConfig;

#[test]
fn publication_rejects_unsafe_bases_unknown_settings_and_empty_index_limits() {
    for base in [
        "//other.test/",
        "javascript:alert(1)",
        "/docs?draft=true",
        "/docs#part",
        "/has space/",
        "https://",
        "https://docs.example.test//reference/",
    ] {
        let config = DocumentationConfig {
            base_url: base.into(),
            ..DocumentationConfig::default()
        };
        assert!(config.validate().is_err(), "{base}");
    }
    assert!(toml::from_str::<DocumentationConfig>("strcit = true").is_err());
    for config in [
        DocumentationConfig {
            full_max_bytes: 0,
            ..DocumentationConfig::default()
        },
        DocumentationConfig {
            index_max_bytes: 0,
            ..DocumentationConfig::default()
        },
    ] {
        assert!(config.validate().is_err());
    }
}
