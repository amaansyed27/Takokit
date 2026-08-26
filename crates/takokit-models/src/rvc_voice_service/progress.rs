use super::*;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

const PROGRESS_LOG_TAIL_BYTES: u64 = 64 * 1024;

impl RvcVoiceService {
    pub(super) fn refresh_job_progress(
        &self,
        voice_id: Uuid,
        job_id: Uuid,
    ) -> TakokitResult<RvcTrainingJob> {
        let mut job = self.store.load_job(voice_id, job_id)?;
        if matches!(
            job.status,
            RvcTrainingJobStatus::Queued | RvcTrainingJobStatus::Running
        ) {
            if let Some(epoch) = read_current_epoch(&job)? {
                if job.current_epoch != Some(epoch) {
                    job.current_epoch = Some(epoch);
                    self.store.save_job(&job)?;
                }
            }
            self.sync_running_project_state(&job)?;
        }
        Ok(job)
    }

    fn sync_running_project_state(&self, job: &RvcTrainingJob) -> TakokitResult<()> {
        let state = match job.stage {
            RvcTrainingStage::ValidateSamples | RvcTrainingStage::Preprocess => {
                RvcVoiceProjectState::Preprocessing
            }
            RvcTrainingStage::ExtractF0 => RvcVoiceProjectState::ExtractingF0,
            RvcTrainingStage::ExtractFeatures => RvcVoiceProjectState::ExtractingFeatures,
            RvcTrainingStage::ReadyToTrain => RvcVoiceProjectState::ReadyToTrain,
            RvcTrainingStage::Train => RvcVoiceProjectState::Training,
            RvcTrainingStage::BuildIndex => RvcVoiceProjectState::BuildingIndex,
            RvcTrainingStage::ValidateArtifacts => RvcVoiceProjectState::ValidatingArtifacts,
            RvcTrainingStage::Complete => return Ok(()),
        };
        let project = self.store.load_id(job.voice_id)?;
        if project.state != state {
            self.store.set_state(job.voice_id, state, None)?;
        }
        Ok(())
    }
}

fn read_current_epoch(job: &RvcTrainingJob) -> TakokitResult<Option<u32>> {
    if job.stage != RvcTrainingStage::Train || !job.log_path.is_file() {
        return Ok(job.current_epoch);
    }
    let mut file = File::open(&job.log_path).map_err(storage)?;
    let length = file.metadata().map_err(storage)?.len();
    if length > PROGRESS_LOG_TAIL_BYTES {
        file.seek(SeekFrom::Start(length - PROGRESS_LOG_TAIL_BYTES))
            .map_err(storage)?;
    }
    let mut text = String::new();
    file.read_to_string(&mut text).map_err(storage)?;
    Ok(text.lines().rev().find_map(parse_epoch_line))
}

fn parse_epoch_line(line: &str) -> Option<u32> {
    let (_, remainder) = line.rsplit_once("====> Epoch:")?;
    remainder.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_upstream_epoch_line() {
        assert_eq!(parse_epoch_line("INFO:root:====> Epoch: 183"), Some(183));
        assert_eq!(parse_epoch_line("not an epoch"), None);
    }
}
