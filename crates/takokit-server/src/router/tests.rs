use super::*;

mod files;
mod models;
mod openai;
mod rvc;
mod system;

#[test]
fn host_header_parser_handles_ipv4_hostname_and_ipv6() {
    assert_eq!(host_header_name("127.0.0.1:5050"), Some("127.0.0.1"));
    assert_eq!(host_header_name("localhost:6060"), Some("localhost"));
    assert_eq!(host_header_name("[::1]:7070"), Some("::1"));
    assert_eq!(host_header_name("::1"), Some("::1"));
    assert_eq!(host_header_name("[::1]:bad"), None);
}

#[test]
fn loopback_detection_accepts_all_local_forms_only() {
    assert!(configured_host_is_loopback("127.0.0.1"));
    assert!(configured_host_is_loopback("127.0.0.2"));
    assert!(configured_host_is_loopback("localhost"));
    assert!(configured_host_is_loopback("::1"));
    assert!(configured_host_is_loopback("[::1]"));
    assert!(!configured_host_is_loopback("0.0.0.0"));
    assert!(!configured_host_is_loopback("192.168.1.20"));
}

#[test]
fn configured_loopback_origin_is_allowed() {
    let config = takokit_core::RuntimeConfig {
        host: "127.0.0.2".into(),
        port: 6060,
        storage_root: std::path::PathBuf::from("."),
    };
    let allowed = loopback_origins(&config);
    assert!(allowed.iter().any(|value| value == "http://127.0.0.2:6060"));
    assert!(allowed.iter().any(|value| value == "http://localhost:6060"));
}
