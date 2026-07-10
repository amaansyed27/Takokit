use anyhow::anyhow;
use serde::{de::DeserializeOwned, Serialize};
use takokit_core::{DaemonIdentity, DaemonMode, RuntimeConfig, SpeechRequest, SpeechResponse};
use takokit_store::LocalStore;

pub struct Client {
    base: String,
}
impl Client {
    pub fn ensure(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<Self> {
        let info = crate::daemon::ensure_running(store, config)?;
        let identity: DaemonIdentity =
            ureq::get(&format!("{}/v1/daemon/identity", config.local_base_url()))
                .call()?
                .into_json()?;
        if identity.mode != DaemonMode::Managed || identity.instance_id != Some(info.instance_id) {
            return Err(anyhow!("managed daemon identity verification failed"));
        }
        Ok(Self {
            base: config.local_base_url(),
        })
    }
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        Ok(ureq::get(&format!("{}{}", self.base, path))
            .call()?
            .into_json()?)
    }
    pub fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        Ok(ureq::post(&format!("{}{}", self.base, path))
            .send_json(serde_json::to_value(body)?)
            .map_err(|e| anyhow!(e.to_string()))?
            .into_json()?)
    }
    pub fn speech(&self, request: SpeechRequest) -> anyhow::Result<SpeechResponse> {
        self.post("/v1/audio/speech", &request)
    }
    pub fn delete(&self, path: &str) -> anyhow::Result<()> {
        ureq::delete(&format!("{}{}", self.base, path))
            .call()
            .map_err(|e| anyhow!(e.to_string()))?;
        Ok(())
    }
}
