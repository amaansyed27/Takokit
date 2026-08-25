use crate::runtime_python_specs::adapter_spec;

#[test]
fn rvc_inference_source_is_not_editable_installed() {
    let spec = adapter_spec("rvc").expect("RVC inference adapter");
    let source = spec.source.expect("pinned RVC source");
    assert!(
        !source.editable,
        "RVC source must not pull its Poetry dependency graph"
    );
    assert!(source.requirement_files.is_empty());
}

#[test]
fn rvc_runtime_dependencies_do_not_compile_fairseq() {
    let spec = adapter_spec("rvc").expect("RVC inference adapter");
    assert!(spec
        .packages
        .iter()
        .any(|item| item.starts_with("transformers")));
    assert!(spec.packages.contains(&"av==12.0.0"));
    assert!(spec
        .packages
        .iter()
        .all(|item| !item.to_ascii_lowercase().contains("fairseq")));
}

#[test]
fn rvc_adapter_uses_managed_transformers_hubert_compatibility() {
    let spec = adapter_spec("rvc").expect("RVC inference adapter");
    let script = spec.script.expect("RVC adapter script");
    assert!(script.contains("install_fairseq_import_stub"));
    assert!(script.contains("load_transformers_hubert"));
    assert!(script.contains("hubert_base"));
    assert!(!script.contains("install_trusted_torch_checkpoint_compat"));
}
