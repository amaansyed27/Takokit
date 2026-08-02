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

#[test]
fn rvc_adapter_resolves_paths_from_fairseq_open_file_objects() {
    let adapter = normalized_adapter();

    assert!(adapter.contains("def checkpoint_candidate(file: object) -> Path | None:"));
    assert!(adapter.contains("getattr(file, \"name\", None)"));
    assert!(adapter.contains("candidate = checkpoint_candidate(file)"));
    assert!(adapter.contains("except (OSError, RuntimeError, TypeError, ValueError):"));
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
