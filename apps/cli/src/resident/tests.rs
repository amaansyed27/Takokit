use super::*;

#[test]
fn installed_icon_path_is_canonical() {
    assert!(
        icon_path().ends_with("resources\\icons\\takokit.ico")
            || icon_path().ends_with("assets\\favicon\\favicon.ico")
    );
}

#[test]
fn configured_api_url_is_copied() {
    let (_, config) = store_and_config();
    assert!(format!("{}/v1", config.local_base_url()).ends_with("/v1"));
}

#[test]
fn quit_stops_only_the_instance_started_by_resident_mode() {
    let owned = Uuid::new_v4();
    assert!(should_stop_owned(Some(owned), Some(owned)));
    assert!(!should_stop_owned(Some(Uuid::new_v4()), Some(owned)));
    assert!(!should_stop_owned(Some(owned), None));
    assert!(!should_stop_owned(None, Some(owned)));
}

#[test]
fn update_available_state_uses_existing_updater_reports() {
    assert_eq!(
        parse_update_version(br#"{"available":true,"offered_version":"0.3.0"}"#, false),
        Some("0.3.0".into())
    );
    assert_eq!(
        parse_update_version(br#"{"checked":true,"available_version":"0.3.0"}"#, true),
        Some("0.3.0".into())
    );
    assert_eq!(
        parse_update_version(br#"{"available":false,"offered_version":"0.2.0"}"#, false),
        None
    );
}
