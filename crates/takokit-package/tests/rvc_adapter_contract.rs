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
fn rvc_adapter_disables_legacy_fairseq_loader_before_upstream_import() {
    let adapter = normalized_adapter();
    let compatibility_call = adapter
        .rfind("install_fairseq_import_stub()")
        .expect("fairseq import stub call");
    let upstream_import = adapter
        .find("from rvc.modules.vc.modules import VC")
        .expect("upstream RVC import");

    assert!(adapter.contains("types.ModuleType(\"fairseq\")"));
    assert!(adapter.contains("checkpoint_utils.load_model_ensemble_and_task = unsupported_loader"));
    assert!(adapter.contains("the legacy fairseq checkpoint loader is disabled"));
    assert!(compatibility_call < upstream_import);
}

#[test]
fn rvc_adapter_injects_local_transformers_hubert_before_conversion() {
    let adapter = normalized_adapter();
    let model_load = adapter
        .find("converter.hubert_model = load_transformers_hubert(")
        .expect("managed Transformers HuBERT load");
    let conversion = adapter
        .find("converter.vc_inference(")
        .expect("upstream RVC conversion call");

    assert!(adapter.contains("from transformers import AutoFeatureExtractor, HubertModel"));
    assert!(adapter.contains("local_files_only=True"));
    assert!(adapter.contains("outputs.hidden_states[9]"));
    assert!(adapter.contains(".last_hidden_state"));
    assert!(adapter.contains("self.final_proj = self.model.final_proj"));
    assert!(model_load < conversion);
}

#[test]
fn rvc_adapter_applies_validated_user_inference_settings() {
    let adapter = normalized_adapter();

    for key in [
        "f0_method",
        "pitch_shift",
        "index_rate",
        "rms_mix_rate",
        "protect",
        "filter_radius",
    ] {
        assert!(
            adapter.contains(&format!("settings[\"{key}\"]")),
            "missing {key}"
        );
    }
    assert!(adapter.contains("F0_METHODS = {\"rmvpe\", \"harvest\", \"crepe\", \"pm\"}"));
    assert!(adapter.contains("if not -24 <= pitch_shift <= 24:"));
    assert!(adapter.contains("if not 0.0 <= index_rate <= 1.0:"));
    assert!(adapter.contains("if not 0.0 <= protect <= 0.5:"));
}

#[test]
fn rvc_adapter_requires_explicit_package_metadata_for_ambiguous_assets() {
    let adapter = normalized_adapter();

    assert!(adapter.contains("rvc.json"));
    assert!(adapter.contains("multiple RVC checkpoints were found"));
    assert!(adapter.contains("multiple RVC indexes were found"));
    assert!(adapter.contains("manifest_verified"));
    assert!(adapter.contains("matched_by_name"));
    assert!(adapter.contains("single_index_unverified"));
}

#[test]
fn rvc_adapter_returns_checkpoint_hashes_and_quality_baseline_state() {
    let adapter = normalized_adapter();

    assert!(adapter.contains("\"checkpoint_sha256\": sha256(model)"));
    assert!(adapter.contains("\"index_sha256\": sha256(index) if index else None"));
    assert!(adapter.contains("\"target_reference_path\""));
    assert!(adapter.contains("\"quality_baseline_ready\""));
    assert!(adapter.contains("effective_settings=settings"));
    assert!(adapter.contains("checkpoint=checkpoint_metadata("));
}