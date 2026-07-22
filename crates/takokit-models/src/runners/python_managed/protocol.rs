//! JSON request/response protocol for managed Python adapters.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub(super) struct ManagedAdapterRequest<'a> {
    pub(super) operation: &'a str,
    pub(super) model_id: &'a str,
    pub(super) model_dir: &'a Path,
    pub(super) cache_dir: &'a Path,
    pub(super) input: Option<&'a str>,
    pub(super) voice: Option<&'a str>,
    pub(super) language: Option<&'a str>,
    pub(super) instruction: Option<&'a str>,
    pub(super) reference_text: Option<&'a str>,
    pub(super) output_path: Option<&'a Path>,
    pub(super) output_dir: Option<&'a Path>,
    pub(super) audio_path: Option<&'a Path>,
    pub(super) target_voice: Option<&'a str>,
    pub(super) dataset_path: Option<&'a Path>,
    pub(super) name: Option<&'a str>,
    pub(super) pitch_shift: Option<i32>,
    pub(super) epochs: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ManagedAdapterResponse {
    pub(super) ok: bool,
    pub(super) output_path: Option<PathBuf>,
    pub(super) bytes: Option<u64>,
    pub(super) sample_rate: Option<u32>,
    pub(super) voice: Option<String>,
    pub(super) text: Option<String>,
    pub(super) status: Option<String>,
    pub(super) log_path: Option<PathBuf>,
    pub(super) error: Option<String>,
}

pub(super) fn decode_adapter_response(stdout: &[u8]) -> Option<ManagedAdapterResponse> {
    String::from_utf8_lossy(stdout)
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str(line.trim()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_response_uses_final_json_after_library_logs() {
        let stdout = b"[NeMo I 00:00:00] loading checkpoint\n                        Transcribing: 100%\n                        {\"ok\":true,\"text\":\"hello from noisy adapter\"}\n";

        let response = decode_adapter_response(stdout).expect("final JSON response");

        assert!(response.ok);
        assert_eq!(response.text.as_deref(), Some("hello from noisy adapter"));
    }
}
