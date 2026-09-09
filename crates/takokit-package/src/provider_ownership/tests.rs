use super::*;

#[test]
fn capture_hardlinks_cache_bytes_into_durable_blob_and_rehydrates() {
    let root = tempfile::tempdir().unwrap();
    let provider = root.path().join("cache/huggingface/hub");
    fs::create_dir_all(&provider).unwrap();
    let before = snapshot_provider_cache(root.path()).unwrap();
    fs::write(provider.join("weights.bin"), vec![7_u8; 4096]).unwrap();
    let ownership = capture_provider_ownership(root.path(), "fixture-model", &before).unwrap();
    assert_eq!(ownership.artifacts.len(), 1);
    assert!(ownership.artifacts[0].blob_path.is_file());
    fs::remove_dir_all(root.path().join("cache/huggingface")).unwrap();
    let restored = ensure_provider_cache_from_ownership(root.path(), "fixture-model").unwrap();
    assert_eq!(restored, 4096);
    assert!(provider.join("weights.bin").is_file());
}

#[test]
fn legacy_migration_is_idempotent_and_journaled() {
    let root = tempfile::tempdir().unwrap();
    let model = root.path().join("models/legacy");
    fs::create_dir_all(&model).unwrap();
    fs::write(model.join(".takokit-prefetch.json"), b"{}").unwrap();
    fs::create_dir_all(root.path().join("cache/torch")).unwrap();
    fs::write(
        root.path().join("cache/torch/checkpoint.bin"),
        vec![3_u8; 128],
    )
    .unwrap();
    let first = migrate_legacy_provider_cache(root.path()).unwrap();
    assert_eq!(first.migrated_models, vec!["legacy"]);
    let second = migrate_legacy_provider_cache(root.path()).unwrap();
    assert!(second.migrated_models.is_empty());
    assert_eq!(second.already_owned_models, vec!["legacy"]);
    assert!(second.journal.is_file());
}

#[test]
fn all_safe_refuses_provider_cache_without_complete_ownership() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("models/legacy")).unwrap();
    fs::write(
        root.path().join("models/legacy/.takokit-prefetch.json"),
        b"{}",
    )
    .unwrap();
    fs::create_dir_all(root.path().join("cache/huggingface")).unwrap();
    fs::write(root.path().join("cache/huggingface/weights"), b"weights").unwrap();
    let report = clean_provider_storage(root.path(), "all-safe", true).unwrap();
    assert!(report
        .retained
        .iter()
        .any(|item| item.category == "huggingface"));
}

#[test]
fn persisted_blob_path_cannot_redirect_rehydration_outside_takokit() {
    let root = tempfile::tempdir().unwrap();
    let provider = root.path().join("cache/huggingface");
    fs::create_dir_all(&provider).unwrap();
    let before = snapshot_provider_cache(root.path()).unwrap();
    fs::write(provider.join("weights.bin"), b"owned-by-takokit").unwrap();
    let mut ownership = capture_provider_ownership(root.path(), "fixture-model", &before).unwrap();
    let external = root.path().join("external.bin");
    fs::write(&external, b"foreign-data").unwrap();
    ownership.artifacts[0].blob_path = external.clone();
    write_model_provider_ownership(root.path(), &ownership).unwrap();
    fs::remove_file(provider.join("weights.bin")).unwrap();

    ensure_provider_cache_from_ownership(root.path(), "fixture-model").unwrap();
    assert_eq!(
        fs::read(provider.join("weights.bin")).unwrap(),
        b"owned-by-takokit"
    );
    assert_eq!(fs::read(external).unwrap(), b"foreign-data");
}

#[cfg(unix)]
#[test]
fn provider_cache_symlink_cannot_escape_provider_root() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let provider = root.path().join("cache/huggingface");
    fs::create_dir_all(&provider).unwrap();
    let external = root.path().join("outside.bin");
    fs::write(&external, b"foreign").unwrap();
    symlink(&external, provider.join("weights.bin")).unwrap();

    let error = snapshot_provider_cache(root.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("escaped its provider root"));
}
