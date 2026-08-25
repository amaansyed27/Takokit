use crate::{
    args::{RvcSampleCommand, RvcVoiceCommand},
    daemon_client::Client,
    output::print_value,
    workspace::CliWorkspace,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::{json, Value};
use takokit_core::{
    AddRvcSamplesRequest, CreateRvcVoiceRequest, ExportRvcVoiceRequest, ImportRvcPackageRequest,
    ImportRvcVoiceRequest, SelectRvcCheckpointRequest, TestRvcVoiceRequest,
    VerifyRvcPackageRequest,
};
use takokit_models::RvcVoiceService;
use takokit_package::{InstalledRegistry, PackageRegistry};
use takokit_store::LocalStore;

pub(crate) async fn run_direct(
    command: RvcVoiceCommand,
    store: &LocalStore,
    packages: &PackageRegistry,
    installed: &InstalledRegistry,
    workspace: Option<&CliWorkspace>,
) -> anyhow::Result<()> {
    let service = RvcVoiceService::new(store.root());
    let value = match command {
        RvcVoiceCommand::Create {
            name,
            consent,
            consent_note,
        } => wrap(
            "rvc_voice",
            service.create(CreateRvcVoiceRequest {
                name,
                consent_affirmed: consent,
                consent_note,
            })?,
        ),
        RvcVoiceCommand::List => wrap("rvc_voices", service.list()?),
        RvcVoiceCommand::Show { voice } => {
            let mut value = serde_json::to_value(service.show(&voice)?)?;
            scrub_detail_job(&mut value);
            json!({"kind":"rvc_voice_detail","data":value})
        }
        RvcVoiceCommand::Samples { voice, command } => match command {
            RvcSampleCommand::Add { paths } => wrap(
                "rvc_samples",
                service.add_samples(&voice, AddRvcSamplesRequest { paths })?,
            ),
            RvcSampleCommand::List => wrap("rvc_samples", service.samples(&voice)?),
            RvcSampleCommand::Remove { sample } => {
                service.remove_sample(&voice, sample)?;
                json!({"kind":"rvc_sample_removal","data":{"voice":voice,"sample":sample,"removed":true}})
            }
        },
        RvcVoiceCommand::Inspect { voice } => {
            wrap("rvc_dataset_inspection", service.inspect_dataset(&voice)?)
        }
        RvcVoiceCommand::Presets => wrap("rvc_training_presets", service.presets()),
        RvcVoiceCommand::Preflight { voice, training } => wrap(
            "rvc_hardware_preflight",
            service.preflight(&voice, training.config()?)?,
        ),
        RvcVoiceCommand::Prepare { voice, training } => public_job(
            "rvc_training_job",
            service.prepare(&voice, training.request()?)?,
        )?,
        RvcVoiceCommand::Train { voice, training } => public_job(
            "rvc_training_job",
            service.start_training(&voice, training.request()?)?,
        )?,
        RvcVoiceCommand::Status { voice } => {
            public_optional_job("rvc_training_job", service.training_status(&voice)?)?
        }
        RvcVoiceCommand::Logs { voice, max_bytes } => json!({
            "kind":"rvc_training_logs",
            "data":{"text":service.training_logs(&voice, max_bytes.min(2 * 1024 * 1024))?}
        }),
        RvcVoiceCommand::Cancel { voice } => {
            public_job("rvc_training_job", service.cancel_training(&voice)?)?
        }
        RvcVoiceCommand::Recover { voice } => {
            public_job("rvc_training_job", service.recover_training(&voice)?)?
        }
        RvcVoiceCommand::Checkpoints { voice } => {
            wrap("rvc_checkpoints", service.checkpoints(&voice)?)
        }
        RvcVoiceCommand::Indexes { voice } => wrap("rvc_indexes", service.indexes(&voice)?),
        RvcVoiceCommand::Activate {
            voice,
            checkpoint,
            index,
        } => wrap(
            "managed_rvc_voice",
            service.select_checkpoint(
                &voice,
                SelectRvcCheckpointRequest {
                    checkpoint_id: checkpoint,
                    index_id: index,
                },
            )?,
        ),
        RvcVoiceCommand::Test { voice, input } => {
            let output_dir = workspace
                .ok_or_else(|| anyhow::anyhow!("RVC voice testing requires a workspace"))?
                .outputs_dir();
            wrap(
                "rvc_voice_test",
                service
                    .test_voice(&voice, input, packages, installed, &output_dir)
                    .await?,
            )
        }
        RvcVoiceCommand::Import {
            checkpoint,
            index,
            name,
            consent,
            consent_note,
        } => wrap(
            "rvc_voice",
            service.import_existing(ImportRvcVoiceRequest {
                checkpoint,
                index,
                name,
                consent_affirmed: consent,
                consent_note,
            })?,
        ),
        RvcVoiceCommand::Export {
            voice,
            output,
            sign,
            include_reference,
        } => {
            let path = service.export_package(
                &voice,
                ExportRvcVoiceRequest {
                    output,
                    sign,
                    include_reference,
                },
            )?;
            json!({"kind":"rvc_voice_package","data":{"path":path}})
        }
        RvcVoiceCommand::Verify { package } => wrap(
            "rvc_package_verification",
            service.verify_package(&package)?,
        ),
        RvcVoiceCommand::ImportPackage {
            package,
            name,
            consent,
            consent_note,
        } => wrap(
            "rvc_voice",
            service.import_package(ImportRvcPackageRequest {
                package,
                name,
                consent_affirmed: consent,
                consent_note,
            })?,
        ),
        RvcVoiceCommand::Remove { voice, dry_run } => {
            wrap("rvc_voice_removal", service.remove(&voice, dry_run)?)
        }
    };
    print_value(&value)
}

pub(crate) fn run_daemon(client: &Client, command: &RvcVoiceCommand) -> anyhow::Result<Value> {
    match command {
        RvcVoiceCommand::Create {
            name,
            consent,
            consent_note,
        } => client.post(
            "/v1/voices/rvc",
            &CreateRvcVoiceRequest {
                name: name.clone(),
                consent_affirmed: *consent,
                consent_note: consent_note.clone(),
            },
        ),
        RvcVoiceCommand::List => client.get("/v1/voices/rvc"),
        RvcVoiceCommand::Show { voice } => client.get(&rvc_path(voice, "")),
        RvcVoiceCommand::Samples { voice, command } => match command {
            RvcSampleCommand::Add { paths } => client.post(
                &rvc_path(voice, "/samples"),
                &AddRvcSamplesRequest {
                    paths: paths.clone(),
                },
            ),
            RvcSampleCommand::List => client.get(&rvc_path(voice, "/samples")),
            RvcSampleCommand::Remove { sample } => {
                client.delete(&rvc_path(voice, &format!("/samples/{sample}")))?;
                Ok(
                    json!({"kind":"rvc_sample_removal","data":{"voice":voice,"sample":sample,"removed":true}}),
                )
            }
        },
        RvcVoiceCommand::Inspect { voice } => {
            client.post(&rvc_path(voice, "/dataset/inspect"), &json!({}))
        }
        RvcVoiceCommand::Presets => client.get("/v1/voices/rvc/presets"),
        RvcVoiceCommand::Preflight { voice, training } => {
            client.post(&rvc_path(voice, "/preflight"), &training.config()?)
        }
        RvcVoiceCommand::Prepare { voice, training } => {
            client.post(&rvc_path(voice, "/prepare"), &training.request()?)
        }
        RvcVoiceCommand::Train { voice, training } => {
            client.post(&rvc_path(voice, "/train"), &training.request()?)
        }
        RvcVoiceCommand::Status { voice } => client.get(&rvc_path(voice, "/train/status")),
        RvcVoiceCommand::Logs { voice, max_bytes } => client.get(&rvc_path(
            voice,
            &format!(
                "/train/logs?max_bytes={}",
                max_bytes.min(&(2 * 1024 * 1024))
            ),
        )),
        RvcVoiceCommand::Cancel { voice } => {
            client.post(&rvc_path(voice, "/train/cancel"), &json!({}))
        }
        RvcVoiceCommand::Recover { voice } => {
            client.post(&rvc_path(voice, "/train/recover"), &json!({}))
        }
        RvcVoiceCommand::Checkpoints { voice } => client.get(&rvc_path(voice, "/checkpoints")),
        RvcVoiceCommand::Indexes { voice } => client.get(&rvc_path(voice, "/indexes")),
        RvcVoiceCommand::Activate {
            voice,
            checkpoint,
            index,
        } => client.post(
            &rvc_path(voice, "/checkpoint"),
            &SelectRvcCheckpointRequest {
                checkpoint_id: *checkpoint,
                index_id: *index,
            },
        ),
        RvcVoiceCommand::Test { voice, input } => client.post(
            &rvc_path(voice, "/test"),
            &TestRvcVoiceRequest {
                input: input.clone(),
                workspace_root: None,
            },
        ),
        RvcVoiceCommand::Import {
            checkpoint,
            index,
            name,
            consent,
            consent_note,
        } => client.post(
            "/v1/voices/rvc/import",
            &ImportRvcVoiceRequest {
                checkpoint: checkpoint.clone(),
                index: index.clone(),
                name: name.clone(),
                consent_affirmed: *consent,
                consent_note: consent_note.clone(),
            },
        ),
        RvcVoiceCommand::Export {
            voice,
            output,
            sign,
            include_reference,
        } => client.post(
            &rvc_path(voice, "/export"),
            &json!({
                "output":output,
                "sign":sign,
                "include_reference":include_reference,
            }),
        ),
        RvcVoiceCommand::Verify { package } => client.post(
            "/v1/voices/rvc/package/verify",
            &VerifyRvcPackageRequest {
                package: package.clone(),
            },
        ),
        RvcVoiceCommand::ImportPackage {
            package,
            name,
            consent,
            consent_note,
        } => client.post(
            "/v1/voices/rvc/package/import",
            &ImportRvcPackageRequest {
                package: package.clone(),
                name: name.clone(),
                consent_affirmed: *consent,
                consent_note: consent_note.clone(),
            },
        ),
        RvcVoiceCommand::Remove { voice, dry_run } => {
            client.delete_json(&format!("{}?dry_run={dry_run}", rvc_path(voice, "")))
        }
    }
}

fn rvc_path(voice: &str, suffix: &str) -> String {
    let encoded = utf8_percent_encode(voice, NON_ALPHANUMERIC).to_string();
    format!("/v1/voices/rvc/{encoded}{suffix}")
}

fn wrap<T: serde::Serialize>(kind: &str, data: T) -> Value {
    json!({"kind":kind,"data":data})
}

fn public_job(kind: &str, job: takokit_core::RvcTrainingJob) -> anyhow::Result<Value> {
    let mut value = serde_json::to_value(job)?;
    scrub_job(&mut value);
    Ok(json!({"kind":kind,"data":value}))
}

fn public_optional_job(
    kind: &str,
    job: Option<takokit_core::RvcTrainingJob>,
) -> anyhow::Result<Value> {
    let value = match job {
        Some(job) => {
            let mut value = serde_json::to_value(job)?;
            scrub_job(&mut value);
            value
        }
        None => Value::Null,
    };
    Ok(json!({"kind":kind,"data":value}))
}

fn scrub_detail_job(value: &mut Value) {
    if let Some(job) = value.get_mut("active_job") {
        scrub_job(job);
    }
}

fn scrub_job(value: &mut Value) {
    let Some(map) = value.as_object_mut() else {
        return;
    };
    for key in [
        "owner_pid",
        "child_pid",
        "log_path",
        "cancellation_requested",
    ] {
        map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_path_encodes_spaces_unicode_and_slashes_as_one_segment() {
        let path = rvc_path("Voice ü / demo", "/checkpoints");
        assert_eq!(
            path,
            "/v1/voices/rvc/Voice%20%C3%BC%20%2F%20demo/checkpoints"
        );
    }

    #[test]
    fn job_scrubber_hides_process_ownership_fields() {
        let mut value = json!({
            "id":"job",
            "owner_pid":123,
            "child_pid":456,
            "log_path":"secret.log",
            "cancellation_requested":false,
            "status":"running"
        });
        scrub_job(&mut value);
        assert!(value.get("owner_pid").is_none());
        assert!(value.get("child_pid").is_none());
        assert!(value.get("log_path").is_none());
        assert_eq!(value["status"], "running");
    }
}
