const RVC_ADAPTER: &str = include_str!("../../../runners/python/rvc_adapter.py");

fn normalized_adapter() -> String {
    RVC_ADAPTER.replace("\r\n", "\n")
}

#[test]
fn rvc_adapter_configures_upstream_model_and_index_roots_before_loading_voice() {
    let adapter = normalized_adapter();
    let configure_call = adapter
        .find("configure_rvc_roots(model_path, index_path)")
        .expect("RVC root configuration call");
    let load_call = adapter
        .find("converter.get_vc(model_path.name)")
        .expect("RVC model load call");

    assert!(adapter.contains("os.environ[\"weight_root\"]"));
    assert!(adapter.contains("os.environ[\"index_root\"]"));
    assert!(configure_call < load_call);
}

#[test]
fn rvc_adapter_passes_the_absolute_audio_path_as_a_string() {
    let adapter = normalized_adapter();
    assert!(adapter.contains("str(source_audio),"));
    assert!(!adapter.contains("\n        source_audio,\n"));
}

#[test]
fn rvc_adapter_normalizes_legacy_pyav_binary_modes_before_importing_upstream() {
    let adapter = normalized_adapter();
    let compatibility_call = adapter
        .rfind("install_pyav_mode_compat()")
        .expect("PyAV compatibility call before upstream imports");
    let upstream_import = adapter
        .find("from rvc.modules.vc.modules import VC")
        .expect("upstream RVC import");

    assert!(adapter.contains("if mode == \"rb\":\n            mode = \"r\""));
    assert!(adapter.contains("elif mode == \"wb\":\n            mode = \"w\""));
    assert!(adapter.contains("av.open = open_compat"));
    assert!(compatibility_call < upstream_import);
}

#[test]
fn rvc_adapter_disables_weights_only_for_the_managed_hubert_checkpoint_only() {
    let adapter = normalized_adapter();
    let compatibility_call = adapter
        .rfind("install_trusted_torch_checkpoint_compat(hubert_path)")
        .expect("trusted HuBERT compatibility call");
    let upstream_import = adapter
        .find("from rvc.modules.vc.modules import VC")
        .expect("upstream RVC import");

    assert!(adapter.contains("candidate == trusted_checkpoint"));
    assert!(adapter.contains("\"weights_only\" not in kwargs"));
    assert!(adapter.contains("kwargs[\"weights_only\"] = False"));
    assert!(adapter.contains("torch.load = load_compat"));
    assert!(!adapter.contains("TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD"));
    assert!(compatibility_call < upstream_import);
}
