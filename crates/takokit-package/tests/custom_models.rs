use std::path::Path;

use takokit_package::{
    custom_model_record, custom_model_records, register_custom_model, ArtifactRole, PackageRegistry,
};

fn write_registry(root: &Path) {
    std::fs::create_dir_all(root.join("models")).unwrap();
    std::fs::create_dir_all(root.join("runners")).unwrap();
    std::fs::write(
        root.join("models").join("qwen3-tts-1.7b-base.toml"),
        r#"
id = "qwen3-tts-1.7b-base"
name = "Qwen3 Base"
family = "qwen3-tts-1.7b-base"
version = "1"
kind = "voice-cloning"
backend = "python-managed"
runner = "takokit-python-managed"
required_adapter = "qwen3_tts"
license = "apache-2.0"
description = "base"

[capabilities]
tts = true
voice_cloning = true

[hardware]
cpu = true
gpu = true
min_ram = "8gb"

[artifacts]
metadata_only = false
weights = []
configs = []
voices = []
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("models").join("whisper-tiny.toml"),
        r#"
id = "whisper-tiny"
name = "Whisper Tiny"
family = "whisper"
version = "1"
kind = "stt"
backend = "whispercpp"
runner = "takokit-whispercpp"
license = "mit"
description = "base"

[capabilities]
stt = true

[hardware]
cpu = true
gpu = false
min_ram = "2gb"

[artifacts]
metadata_only = false
weights = []
configs = []
voices = []
"#,
    )
    .unwrap();
}

#[test]
fn registers_and_resolves_pinned_qwen_custom_model() {
    let temp = tempfile::tempdir().unwrap();
    let registry_root = temp.path().join("registry");
    let takokit_root = temp.path().join("home");
    write_registry(&registry_root);
    let registry = PackageRegistry::new(&registry_root)
        .with_custom_models_dir(takokit_root.join("manifests/custom/models"));
    let input = temp.path().join("custom.toml");
    std::fs::write(
        &input,
        r#"
schema_version = 1
id = "amaan-qwen"
name = "Amaan Qwen"
extends = "qwen3-tts-1.7b-base"
version = "1.0.0"
license = "apache-2.0"
description = "Pinned local fine-tune."

[source]
provider = "hugging-face"
repository = "example/amaan-qwen"
revision = "0123456789abcdef0123456789abcdef01234567"

[artifacts]
metadata_only = false
weights = []
configs = []
voices = []
"#,
    )
    .unwrap();

    let record = register_custom_model(&takokit_root, &registry, &input).unwrap();
    assert_eq!(record.canonical_reference, "local/amaan-qwen:latest");
    assert_eq!(record.manifest.family, "qwen3-tts-1.7b-base");
    assert_eq!(
        record.manifest.required_adapter.as_deref(),
        Some("qwen3_tts")
    );

    let resolved = registry.model("local/amaan-qwen:latest").unwrap();
    assert_eq!(resolved.id, "amaan-qwen");
    assert!(resolved.capabilities.tts);
    assert!(resolved.capabilities.voice_cloning);
    assert_eq!(
        custom_model_records(&takokit_root, &registry)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        custom_model_record(&takokit_root, &registry, "amaan-qwen")
            .unwrap()
            .manifest
            .id,
        "amaan-qwen"
    );
}

#[test]
fn accepts_one_checksum_pinned_whispercpp_artifact() {
    let temp = tempfile::tempdir().unwrap();
    let registry_root = temp.path().join("registry");
    let takokit_root = temp.path().join("home");
    write_registry(&registry_root);
    let registry = PackageRegistry::new(&registry_root)
        .with_custom_models_dir(takokit_root.join("manifests/custom/models"));
    let input = temp.path().join("whisper.toml");
    std::fs::write(
        &input,
        r#"
schema_version = 1
id = "amaan-whisper"
name = "Amaan Whisper"
extends = "whisper-tiny"
version = "1"
license = "mit"
description = "Custom whisper.cpp checkpoint."

[artifacts]
metadata_only = false
configs = []
voices = []

[[artifacts.weights]]
name = "amaan-whisper.bin"
url = "https://example.com/amaan-whisper.bin"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
bytes = 123
role = "model"
"#,
    )
    .unwrap();

    let record = register_custom_model(&takokit_root, &registry, &input).unwrap();
    assert_eq!(record.manifest.family, "whisper");
    assert_eq!(
        record.manifest.artifacts.weights[0].role,
        ArtifactRole::Model
    );
}

#[test]
fn rejects_unpinned_or_unsafe_custom_models() {
    let temp = tempfile::tempdir().unwrap();
    let registry_root = temp.path().join("registry");
    let takokit_root = temp.path().join("home");
    write_registry(&registry_root);
    let registry = PackageRegistry::new(&registry_root)
        .with_custom_models_dir(takokit_root.join("manifests/custom/models"));
    let input = temp.path().join("bad.toml");
    std::fs::write(
        &input,
        r#"
schema_version = 1
id = "../unsafe"
name = "Unsafe"
extends = "qwen3-tts-1.7b-base"
version = "1"
license = "unknown"
description = "bad"

[source]
provider = "hugging-face"
repository = "example/model"
revision = "main"

[artifacts]
metadata_only = false
weights = []
configs = []
voices = []
"#,
    )
    .unwrap();
    assert!(register_custom_model(&takokit_root, &registry, &input).is_err());
}
