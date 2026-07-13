use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use takokit_core::{TakokitError, TakokitResult, VoiceProfile};

#[derive(Debug, Clone)]
pub struct VoiceProfileStore {
    root: PathBuf,
}

impl VoiceProfileStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn create(
        &self,
        name: &str,
        model_id: &str,
        sample_path: &Path,
        consent_affirmed: bool,
        consent_note: Option<String>,
    ) -> TakokitResult<VoiceProfile> {
        if !consent_affirmed {
            return Err(TakokitError::InvalidRequest(format!(
                "voice cloning requires explicit consent for {}",
                name.trim()
            )));
        }
        if !sample_path.is_file() {
            return Err(TakokitError::InvalidRequest(format!(
                "voice reference does not exist: {}",
                sample_path.display()
            )));
        }
        let id = voice_id(name)?;
        let directory = self.root.join(&id);
        if directory.exists() {
            return Err(TakokitError::InvalidRequest(format!(
                "voice profile {id} already exists"
            )));
        }
        std::fs::create_dir_all(&directory).map_err(storage_error)?;
        let extension = sample_path
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("wav");
        let reference = directory.join(format!("reference.{extension}"));
        std::fs::copy(sample_path, &reference).map_err(storage_error)?;
        let profile = VoiceProfile {
            id,
            name: name.trim().to_string(),
            model_id: model_id.trim().to_string(),
            sample_path: reference,
            created_at: now(),
            consent_affirmed,
            consent_note,
        };
        std::fs::write(
            directory.join("profile.json"),
            serde_json::to_vec_pretty(&profile).map_err(storage_error)?,
        )
        .map_err(storage_error)?;
        Ok(profile)
    }

    pub fn get(&self, id: &str) -> TakokitResult<VoiceProfile> {
        let id = voice_id(id)?;
        let path = self.root.join(id).join("profile.json");
        let source = std::fs::read_to_string(&path).map_err(|error| {
            TakokitError::Storage(format!(
                "could not read voice profile {}: {error}",
                path.display()
            ))
        })?;
        serde_json::from_str(&source).map_err(storage_error)
    }

    pub fn list(&self) -> TakokitResult<Vec<VoiceProfile>> {
        std::fs::create_dir_all(&self.root).map_err(storage_error)?;
        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(&self.root).map_err(storage_error)? {
            let entry = entry.map_err(storage_error)?;
            if !entry.path().is_dir() {
                continue;
            }
            let path = entry.path().join("profile.json");
            if !path.is_file() {
                continue;
            }
            let source = std::fs::read_to_string(path).map_err(storage_error)?;
            profiles.push(serde_json::from_str(&source).map_err(storage_error)?);
        }
        profiles.sort_by(|left: &VoiceProfile, right: &VoiceProfile| {
            right.created_at.cmp(&left.created_at)
        });
        Ok(profiles)
    }

    pub fn resolve_reference(&self, id_or_path: &str) -> TakokitResult<PathBuf> {
        let path = PathBuf::from(id_or_path);
        if path.is_file() {
            return Ok(path);
        }
        let profile = self.get(id_or_path)?;
        if !profile.sample_path.is_file() {
            return Err(TakokitError::Storage(format!(
                "voice reference is missing: {}",
                profile.sample_path.display()
            )));
        }
        Ok(profile.sample_path)
    }
}

fn voice_id(name: &str) -> TakokitResult<String> {
    let mut id = String::new();
    let mut separator = false;
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            id.push(character.to_ascii_lowercase());
            separator = false;
        } else if matches!(character, '-' | '_' | ' ') && !separator && !id.is_empty() {
            id.push('-');
            separator = true;
        }
    }
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        return Err(TakokitError::InvalidRequest(
            "voice name must contain letters or numbers".to_string(),
        ));
    }
    Ok(id)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn storage_error(error: impl std::fmt::Display) -> TakokitError {
    TakokitError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_resolves_consent_backed_voice_profile() {
        let root = std::env::temp_dir().join(format!("takokit-voice-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let sample = root.join("sample.wav");
        std::fs::write(&sample, b"RIFF").unwrap();
        let store = VoiceProfileStore::new(root.join("voices"));
        let profile = store
            .create("My Voice", "chatterbox", &sample, true, Some("owner".into()))
            .unwrap();
        assert_eq!(profile.id, "my-voice");
        assert_eq!(store.resolve_reference("my-voice").unwrap(), profile.sample_path);
        assert_eq!(store.list().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_profile_without_consent() {
        let root = std::env::temp_dir().join(format!("takokit-voice-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let sample = root.join("sample.wav");
        std::fs::write(&sample, b"RIFF").unwrap();
        let store = VoiceProfileStore::new(root.join("voices"));
        assert!(store
            .create("No Consent", "chatterbox", &sample, false, None)
            .is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
