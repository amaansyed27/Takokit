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

fn initializing_python_managed_runner_writes_adapter_slots() {
    let temp = tempfile::tempdir().expect("tempdir");

    let registry = PackageRegistry::bundled();

    let manifest = registry
        .runner("takokit-python-managed")
        .expect("python runner");

    let installed = InstalledRegistry::new(temp.path().join("manifests"));

    initialize_runner_runtime(temp.path(), &installed, &manifest).expect("runtime init");

    let adapters = temp
        .path()
        .join("runners")
        .join("python-managed")
        .join("adapters");

    for adapter in ["qwen3_tts", "chatterbox", "f5_tts", "rvc"] {
        assert!(
            adapters.join(adapter).join("adapter.toml").is_file(),
            "missing {adapter} adapter manifest"
        );
    }
}
