use serde::Serialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CacheEntryReport {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) files: u64,
    pub(crate) logical_bytes: u64,
    pub(crate) classification: String,
    pub(crate) cleanable: bool,
}

#[derive(Debug, Default)]
struct CacheTotals {
    files: u64,
    logical_bytes: u64,
}

pub(crate) fn inspect_cache_entries(root: &Path) -> io::Result<Vec<CacheEntryReport>> {
    let cache_root = root.join("cache");
    if !cache_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&cache_root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let totals = scan_path(&path)?;
        let (classification, cleanable) = classify(&name);
        entries.push(CacheEntryReport {
            name,
            path,
            files: totals.files,
            logical_bytes: totals.logical_bytes,
            classification: classification.to_string(),
            cleanable,
        });
    }

    entries.sort_by(|left, right| {
        right
            .logical_bytes
            .cmp(&left.logical_bytes)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

fn classify(name: &str) -> (&'static str, bool) {
    match name.to_ascii_lowercase().as_str() {
        "uv" => ("package cache; safely cleanable", true),
        "downloads" => (
            "download staging or partial pulls; protected pending provider-aware GC",
            false,
        ),
        "huggingface" | "torch" | "coqui" | "modelscope" | "openvoice" => (
            "may contain active installed-model checkpoints; protected",
            false,
        ),
        _ => ("unknown cache ownership; protected", false),
    }
}

fn scan_path(path: &Path) -> io::Result<CacheTotals> {
    if !path.exists() {
        return Ok(CacheTotals::default());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(CacheTotals::default());
    }
    if metadata.is_file() {
        return Ok(CacheTotals {
            files: 1,
            logical_bytes: metadata.len(),
        });
    }

    let mut totals = CacheTotals::default();
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let child = scan_path(&entry?.path())?;
            totals.files = totals.files.saturating_add(child.files);
            totals.logical_bytes = totals.logical_bytes.saturating_add(child.logical_bytes);
        }
    }
    Ok(totals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_entries_are_reported_and_only_uv_is_cleanable() {
        let root = tempfile::tempdir().expect("temporary directory");
        let uv = root.path().join("cache/uv");
        let huggingface = root.path().join("cache/huggingface/hub");
        let unknown = root.path().join("cache/custom");
        fs::create_dir_all(&uv).expect("uv directory");
        fs::create_dir_all(&huggingface).expect("huggingface directory");
        fs::create_dir_all(&unknown).expect("unknown directory");
        fs::write(uv.join("wheel"), vec![1_u8; 16]).expect("uv fixture");
        fs::write(huggingface.join("weights"), vec![2_u8; 32]).expect("hf fixture");
        fs::write(unknown.join("data"), vec![3_u8; 8]).expect("unknown fixture");

        let entries = inspect_cache_entries(root.path()).expect("cache inventory");
        let uv = entries
            .iter()
            .find(|entry| entry.name == "uv")
            .expect("uv report");
        let huggingface = entries
            .iter()
            .find(|entry| entry.name == "huggingface")
            .expect("huggingface report");
        let unknown = entries
            .iter()
            .find(|entry| entry.name == "custom")
            .expect("unknown report");

        assert!(uv.cleanable);
        assert_eq!(uv.logical_bytes, 16);
        assert!(!huggingface.cleanable);
        assert!(huggingface
            .classification
            .contains("active installed-model"));
        assert!(!unknown.cleanable);
        assert!(unknown.classification.contains("unknown"));
    }
}
