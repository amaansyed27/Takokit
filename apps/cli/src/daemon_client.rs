use anyhow::anyhow;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{de::DeserializeOwned, Serialize};
use takokit_core::{DaemonIdentity, DaemonMode, RuntimeConfig, SpeechRequest, SpeechResponse};
use takokit_store::LocalStore;

use crate::workspace::{SESSION_ENV, WORKSPACE_ENV};

pub struct Client {
    base: String,
    workspace_header: Option<String>,
    session_header: Option<String>,
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
            workspace_header: std::env::var(WORKSPACE_ENV)
                .ok()
                .map(|value| utf8_percent_encode(&value, NON_ALPHANUMERIC).to_string()),
            session_header: std::env::var(SESSION_ENV).ok(),
        })
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        Ok(self
            .headers(ureq::get(&format!("{}{}", self.base, path)))
            .call()?
            .into_json()?)
    }

    pub fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        Ok(self
            .headers(ureq::post(&format!("{}{}", self.base, path)))
            .send_json(serde_json::to_value(body)?)
            .map_err(|error| anyhow!(error.to_string()))?
            .into_json()?)
    }

    pub fn speech(&self, request: SpeechRequest) -> anyhow::Result<SpeechResponse> {
        self.post("/v1/audio/speech", &request)
    }

    pub fn delete(&self, path: &str) -> anyhow::Result<()> {
        self.headers(ureq::delete(&format!("{}{}", self.base, path)))
            .call()
            .map_err(|error| anyhow!(error.to_string()))?;
        Ok(())
    }

    fn headers(&self, mut request: ureq::Request) -> ureq::Request {
        if let Some(workspace) = &self.workspace_header {
            request = request.set("X-Takokit-Workspace", workspace);
        }
        if let Some(session) = &self.session_header {
            request = request.set("X-Takokit-Session", session);
        }
        request
    }
}
