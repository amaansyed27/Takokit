use super::*;

#[test]
fn python_managed_runner_layout_resolves_under_takokit_root() {
    let root = PathBuf::from("/tmp/takokit-test-root");
    let layout = python_managed_runner_layout(&root);

    assert_eq!(layout.root, root.join("runners").join("python-managed"));
    assert_eq!(layout.runtime, layout.root.join("runtime"));
    assert_eq!(layout.env, layout.root.join("env"));
    assert_eq!(layout.packages, layout.root.join("packages"));
    assert_eq!(layout.wheels, layout.root.join("wheels"));
    assert_eq!(layout.logs, layout.root.join("logs"));
    assert_eq!(layout.manifests, layout.root.join("manifests"));
    assert_eq!(layout.cache, layout.root.join("cache"));
    assert_eq!(layout.adapters, layout.root.join("adapters"));
}

#[test]
fn finds_managed_uv_before_path_lookup() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
    let uv = temp.path().join("tools").join("uv").join(executable);
    std::fs::create_dir_all(uv.parent().expect("parent")).expect("tools dir");
    std::fs::write(&uv, b"fixture").expect("uv fixture");

    assert_eq!(find_uv(temp.path()), Some(uv));
}

#[test]
fn writing_python_managed_adapter_slots_is_offline_and_deterministic() {
    let temp = tempfile::tempdir().expect("tempdir");
    let layout = python_managed_runner_layout(temp.path());

    crate::runtime_python::write_python_adapter_manifests(&layout)
        .expect("write adapter manifests");

    for adapter in [
        "qwen3_tts",
        "chatterbox",
        "f5_tts",
        "cosyvoice2",
        "dia",
        "fish_speech",
        "openvoice",
        "gpt_sovits",
        "rvc",
    ] {
        assert!(
            layout.adapters.join(adapter).join("adapter.toml").is_file(),
            "missing {adapter} adapter manifest"
        );
    }
    assert!(!layout.env.join("venv").exists());
    assert!(!temp.path().join("tools").join("uv").exists());
}
