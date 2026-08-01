const RVC_ADAPTER: &str = include_str!("../../../runners/python/rvc_adapter.py");

#[test]
fn rvc_adapter_configures_upstream_model_and_index_roots_before_loading_voice() {
    let configure_call = RVC_ADAPTER
        .find("configure_rvc_roots(model_path, index_path)")
        .expect("RVC root configuration call");
    let load_call = RVC_ADAPTER
        .find("converter.get_vc(model_path.name)")
        .expect("RVC model load call");

    assert!(RVC_ADAPTER.contains("os.environ[\"weight_root\"]"));
    assert!(RVC_ADAPTER.contains("os.environ[\"index_root\"]"));
    assert!(configure_call < load_call);
}

#[test]
fn rvc_adapter_passes_the_absolute_audio_path_as_a_string() {
    assert!(RVC_ADAPTER.contains("str(source_audio),"));
    assert!(!RVC_ADAPTER.contains("\n        source_audio,\n"));
}

#[test]
fn rvc_adapter_normalizes_legacy_pyav_binary_modes_before_importing_upstream() {
    let compatibility_call = RVC_ADAPTER
        .rfind("install_pyav_mode_compat()")
        .expect("PyAV compatibility call before upstream imports");
    let upstream_import = RVC_ADAPTER
        .find("from rvc.modules.vc.modules import VC")
        .expect("upstream RVC import");

    assert!(RVC_ADAPTER.contains("if mode == \"rb\":\n            mode = \"r\""));
    assert!(RVC_ADAPTER.contains("elif mode == \"wb\":\n            mode = \"w\""));
    assert!(RVC_ADAPTER.contains("av.open = open_compat"));
    assert!(compatibility_call < upstream_import);
}
