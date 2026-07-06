use async_trait::async_trait;
use std::path::{Path, PathBuf};
use takokit_core::{SpeechRequest, SpeechResponse, TakokitResult};

#[async_trait]
pub trait TextToSpeechEngine: Send + Sync {
    fn id(&self) -> &'static str;
    async fn synthesize(
        &self,
        request: SpeechRequest,
        output_dir: &Path,
    ) -> TakokitResult<SpeechResponse>;
}

#[async_trait]
pub trait SpeechToTextEngine: Send + Sync {
    async fn transcribe(&self, audio_path: PathBuf) -> TakokitResult<String>;
}

#[async_trait]
pub trait VoiceCloneEngine: Send + Sync {
    async fn clone_voice(&self, sample_path: PathBuf, name: String) -> TakokitResult<()>;
}

#[async_trait]
pub trait VoiceTrainingEngine: Send + Sync {
    async fn train_voice(&self, samples_path: PathBuf, name: String) -> TakokitResult<()>;
}

#[async_trait]
pub trait VoiceConversionEngine: Send + Sync {
    async fn convert_voice(
        &self,
        input_path: PathBuf,
        target_voice: String,
    ) -> TakokitResult<PathBuf>;
}
