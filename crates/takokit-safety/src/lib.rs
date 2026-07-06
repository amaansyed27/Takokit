use takokit_core::{TakokitError, TakokitResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentRecord {
    pub voice_name: String,
    pub affirmed_by_user: bool,
}

pub fn require_voice_clone_consent(record: &ConsentRecord) -> TakokitResult<()> {
    if record.affirmed_by_user {
        Ok(())
    } else {
        Err(TakokitError::InvalidRequest(format!(
            "voice cloning requires explicit consent for {}",
            record.voice_name
        )))
    }
}
