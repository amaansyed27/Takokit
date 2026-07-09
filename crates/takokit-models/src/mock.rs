use crate::TextToSpeechEngine;
use async_trait::async_trait;
use std::path::Path;
use takokit_audio::{write_silence_wav, WavSpec};
use takokit_core::{SpeechRequest, SpeechResponse, TakokitError, TakokitResult};
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct MockTextToSpeechEngine;

#[async_trait]
impl TextToSpeechEngine for MockTextToSpeechEngine {
    fn id(&self) -> &'static str {
        "mock-tts"
    }

    async fn synthesize(
        &self,
        request: SpeechRequest,
        output_dir: &Path,
    ) -> TakokitResult<SpeechResponse> {
        if request.input.trim().is_empty() {
            return Err(TakokitError::InvalidRequest(
                "speech input cannot be empty".to_string(),
            ));
        }

        std::fs::create_dir_all(output_dir)
            .map_err(|error| TakokitError::Storage(error.to_string()))?;
        let id = Uuid::new_v4();
        let output_path = output_dir.join(format!("speech-{id}.wav"));
        let bytes = write_silence_wav(&output_path, 600, WavSpec::default())?;

        Ok(SpeechResponse {
            id,
            model: request.model,
            voice: request.voice,
            engine: self.id().to_string(),
            output_path,
            content_type: "audio/wav".to_string(),
            bytes,
            sample_rate: Some(WavSpec::default().sample_rate),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_tts_writes_test_wav() {
        let dir = std::env::temp_dir().join("takokit-mock-tts-test");
        let engine = MockTextToSpeechEngine;
        let response = engine
            .synthesize(
                SpeechRequest {
                    model: "mock-tts".to_string(),
                    input: "hello".to_string(),
                    voice: Some("default".to_string()),
                    response_format: Some("wav".to_string()),
                },
                &dir,
            )
            .await
            .expect("speech response");

        assert_eq!(response.engine, "mock-tts");
        assert!(response.output_path.exists());
        let _ = std::fs::remove_file(response.output_path);
        let _ = std::fs::remove_dir_all(dir);
    }
}
