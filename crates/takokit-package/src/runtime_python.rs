//! Managed Python runtime and model-adapter lifecycle.

use crate::{
    runtime_command::{
        run_logged_command, run_logged_command_with_env, runner_python_path, PathOrArg,
    },
    runtime_python_specs::{adapter_spec, AdapterSourceSpec, AdapterSpec, ADAPTER_SPECS},
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::{Path, PathBuf};

mod prefetch;
pub(crate) use prefetch::prefetch_python_adapter_model;

const SHARED_RUNTIME_VERSION: &str = "shared-python-v1";
const SHARED_RUNTIME_PACKAGES: &[&str] = &[
    "torch",
    "torchaudio",
    "huggingface_hub[hf_xet]",
    "numpy",
    "scipy",
    "soundfile",
    "packaging",
];

pub(crate) fn write_python_adapter_manifests(
    layout: &PythonManagedRunnerLayout,
) -> PackageResult<()> {
    for spec in ADAPTER_SPECS {
        let adapter_dir = layout.adapters.join(spec.id);
        std::fs::create_dir_all(&adapter_dir)?;
        let manifest = adapter_dir.join("adapter.toml");
        if !manifest.is_file() {
            write_adapter_record(
                &manifest,
                &AdapterRecord {
                    id: spec.id.to_string(),
                    model_family: spec.model_family.to_string(),
                    state: AdapterLifecycleState::NotInstalled,
                    dependency_strategy: "shared-takokit-python-base-with-isolated-overlay".to_string(),
                    input_contract: "typed JSON request on stdin".to_string(),
                    output_contract: "typed JSON response on stdout".to_string(),
                    logs: "install.log".to_string(),
                    notes: spec.note.to_string(),
                },
            )?;
        }
    }
    Ok(())
}

pub fn python_adapter_records(takokit_root: &Path) -> PackageResult<Vec<AdapterRecord>> {
    let layout = python_managed_runner_layout(takokit_root);
    let mut records = Vec::new();
    if !layout.adapters.is_dir() {
        return Ok(records);
    }
    let mut entries = std::fs::read_dir(&layout.adapters)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path().join("adapter.toml");
        if path.is_file() {
            let source = std::fs::read_to_string(path)?;
            records.push(toml::from_str::<AdapterRecord>(&source)?);
        }
    }
    Ok(records)
}

pub(crate) fn python_adapter_is_current(takokit_root: &Path, adapter: &str) -> bool {
    let Some(spec) = adapter_spec(adapter) else {
        return false;
    };
    let Some(expected_script) = spec.script else {
        return false;
    };
    let adapter_dir = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter);
    let deployed_script = adapter_dir.join(format!("{adapter}.py"));
    let shared_marker = adapter_dir.join(".takokit-shared-runtime");
    let venv = adapter_dir.join("venv");
    python_adapter_record(takokit_root, adapter)
        .is_ok_and(|record| record.state == AdapterLifecycleState::Ready)
        && std::fs::read_to_string(deployed_script).is_ok_and(|script| script == expected_script)
        && std::fs::read_to_string(shared_marker)
            .is_ok_and(|version| version.trim() == shared_runtime_identity(spec.python))
        && venv_inherits_shared_packages(&venv)
}

pub fn python_adapter_record(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let path = python_managed_runner_layout(takokit_root)
        .adapters
        .join(adapter)
        .join("adapter.toml");
    let source = std::fs::read_to_string(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: format!(
                "adapter is not available; run `takokit runner install takokit-python-managed`: {}",
                path.display()
            ),
        },
        _ => PackageError::Io(error),
    })?;
    Ok(toml::from_str::<AdapterRecord>(&source)?)
}

pub fn install_python_adapter(takokit_root: &Path, adapter: &str) -> PackageResult<AdapterRecord> {
    let layout = python_managed_runner_layout(takokit_root);
    write_python_adapter_manifests(&layout)?;
    let manifest_path = layout.adapters.join(adapter).join("adapter.toml");
    let mut record = python_adapter_record(takokit_root, adapter)?;
    let reset_environment = record.state == AdapterLifecycleState::Failed;
    record.state = AdapterLifecycleState::Installing;
    record.notes = "Takokit is installing a lightweight adapter overlay on the shared Python base.".to_string();
    write_adapter_record(&manifest_path, &record)?;

    let result = adapter_spec(adapter)
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: adapter.to_string(),
            reason: "unknown managed adapter".to_string(),
        })
        .and_then(|spec| install_adapter_spec(takokit_root, &layout, spec, reset_environment));
    match result {
        Ok(note) => {
            record.state = AdapterLifecycleState::Ready;
            record.notes = note;
            write_adapter_record(&manifest_path, &record)?;
            Ok(record)
        }
        Err(error) => {
            record.state = AdapterLifecycleState::Failed;
            record.notes = format!("Adapter install failed: {error}");
            write_adapter_record(&manifest_path, &record)?;
            Err(error)
        }
    }
}

pub(crate) fn install_python_managed_runtime(
    takokit_root: &Path,
    installed_registry: &InstalledRegistry,
    manifest: &RunnerManifest,
) -> PackageResult<PullReport> {
    let layout = python_managed_runner_layout(takokit_root);
    for path in [
        &layout.root,
        &layout.runtime,
        &layout.env,
        &layout.packages,
        &layout.wheels,
        &layout.logs,
        &layout.manifests,
        &layout.cache,
        &layout.adapters,
    ] {
        std::fs::create_dir_all(path)?;
    }
    write_python_adapter_manifests(&layout)?;
    let uv = bootstrap_uv(takokit_root)?;
    installed_registry.install_runner_runtime(
        manifest,
        RunnerLifecycleState::Ready,
        format!(
            "Managed Python runtime is ready at {} using {}. A shared dependency base is installed once for each Python ABI when its first adapter is pulled.",
            layout.root.display(),
            uv.display()
        ),
    )
}

fn install_adapter_spec(
    takokit_root: &Path,
    layout: &PythonManagedRunnerLayout,
    spec: &AdapterSpec,
    reset_environment: bool,
) -> PackageResult<String> {
    let script = spec
        .script
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: format!("{} has no adapter script", spec.model_family),
        })?;
    if spec.packages.is_empty() && spec.source.is_none() {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: spec.id.to_string(),
            reason: "adapter has no dependency or source installation plan".to_string(),
        });
    }

    let adapter_dir = layout.adapters.join(spec.id);
    std::fs::create_dir_all(&adapter_dir)?;
    let venv = adapter_dir.join("venv");
    let log = adapter_dir.join("install.log");
    let shared_python = ensure_shared_python_runtime(takokit_root, layout, spec.python)?;
    let must_migrate = !venv_inherits_shared_packages(&venv)
        || std::fs::read_to_string(adapter_dir.join(".takokit-shared-runtime"))
            .map(|value| value.trim() != shared_runtime_identity(spec.python))
            .unwrap_or(true);
    if (reset_environment || must_migrate) && venv.exists() {
        std::fs::remove_dir_all(&venv)?;
    }
    let uv = bootstrap_uv(takokit_root)?;
    run_logged_uv_command(
        takokit_root,
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            shared_python.into(),
            "--system-site-packages".into(),
            "--allow-existing".into(),
            venv.clone().into(),
        ],
    )?;
    let python = runner_python_path(&venv).ok_or_else(|| PackageError::ArtifactInstallFailed {
        artifact: spec.id.to_string(),
        reason: format!(
            "adapter environment has no Python executable: {}",
            venv.display()
        ),
    })?;

    let source_dir = match spec.source.as_ref() {
        Some(source) => Some(install_adapter_source(&adapter_dir, &log, source)?),
        None => None,
    };
    if !spec.packages.is_empty() {
        uv_pip_install(
            takokit_root,
            &uv,
            &python,
            &log,
            spec.packages.iter().map(|item| (*item).into()),
        )?;
    }
    if !spec.no_deps_packages.is_empty() {
        uv_pip_install(
            takokit_root,
            &uv,
            &python,
            &log,
            std::iter::once("--no-deps".into())
                .chain(spec.no_deps_packages.iter().map(|item| (*item).into())),
        )?;
    }
    if let (Some(source), Some(source_dir)) = (spec.source.as_ref(), source_dir.as_ref()) {
        for requirements in source.requirement_files {
            let path = source_dir.join(requirements);
            if !path.is_file() {
                return Err(PackageError::ArtifactInstallFailed {
                    artifact: spec.id.to_string(),
                    reason: format!("required dependency file is missing: {}", path.display()),
                });
            }
            uv_pip_install(
                takokit_root,
                &uv,
                &python,
                &log,
                ["-r".into(), path.into()].into_iter(),
            )?;
        }
        if source.editable {
            uv_pip_install(
                takokit_root,
                &uv,
                &python,
                &log,
                ["-e".into(), source_dir.clone().into()].into_iter(),
            )?;
        }
    }

    std::fs::write(adapter_dir.join(format!("{}.py", spec.id)), script)?;
    std::fs::write(
        adapter_dir.join(".takokit-shared-runtime"),
        shared_runtime_identity(spec.python),
    )?;
    Ok(format!(
        "Ready. {} Shared Python {} with adapter overlay: {}. Source: {}. Install log: {}",
        spec.note,
        spec.python,
        venv.display(),
        source_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "package-managed".to_string()),
        log.display()
    ))
}


fn shared_runtime_identity(python: &str) -> String {
    format!("{SHARED_RUNTIME_VERSION}-py{python}")
}

fn shared_runtime_dir(layout: &PythonManagedRunnerLayout, python: &str) -> PathBuf {
    layout
        .env
        .join(format!("shared-python-{}", python.replace('.', "_")))
}

fn venv_inherits_shared_packages(venv: &Path) -> bool {
    std::fs::read_to_string(venv.join("pyvenv.cfg"))
        .is_ok_and(|config| {
            config.lines().any(|line| {
                let normalized = line.replace(' ', "").to_ascii_lowercase();
                normalized == "include-system-site-packages=true"
            })
        })
}

fn managed_base_python(venv: &Path, takokit_root: &Path) -> PackageResult<PathBuf> {
    let config_path = venv.join("pyvenv.cfg");
    let config =
        std::fs::read_to_string(&config_path).map_err(|error| PackageError::ArtifactInstallFailed {
            artifact: "shared managed Python".to_string(),
            reason: format!("could not read {}: {error}", config_path.display()),
        })?;

    let value = |key: &str| {
        config.lines().find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            candidate.trim().eq_ignore_ascii_case(key).then(|| value.trim())
        })
    };
    let mut candidates = Vec::new();
    for key in ["base-executable", "executable"] {
        if let Some(path) = value(key) {
            candidates.push(PathBuf::from(path));
        }
    }
    if let Some(home) = value("home").map(PathBuf::from) {
        if cfg!(windows) {
            candidates.push(home.join("python.exe"));
        } else {
            candidates.push(home.join("bin").join("python3"));
            candidates.push(home.join("bin").join("python"));
        }
    }

    let managed_root = takokit_root.join("tools").join("python");
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file() && candidate.starts_with(&managed_root))
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: "shared managed Python".to_string(),
            reason: format!(
                "{} did not identify a Takokit-owned base interpreter below {}; refusing to modify a system Python",
                config_path.display(),
                managed_root.display()
            ),
        })
}

fn ensure_shared_python_runtime(
    takokit_root: &Path,
    layout: &PythonManagedRunnerLayout,
    python_version: &str,
) -> PackageResult<PathBuf> {
    let shared_dir = shared_runtime_dir(layout, python_version);
    let bootstrap_venv = shared_dir.join("bootstrap");
    let marker = shared_dir.join(".takokit-shared-runtime");
    let log = shared_dir.join("install.log");
    std::fs::create_dir_all(&shared_dir)?;
    let uv = bootstrap_uv(takokit_root)?;

    run_logged_uv_command(
        takokit_root,
        &log,
        &uv,
        &[
            "venv".into(),
            "--python".into(),
            python_version.into(),
            "--allow-existing".into(),
            bootstrap_venv.clone().into(),
        ],
    )?;
    let base_python = managed_base_python(&bootstrap_venv, takokit_root)?;
    let identity = shared_runtime_identity(python_version);
    if std::fs::read_to_string(&marker)
        .map(|value| value.trim() == identity)
        .unwrap_or(false)
    {
        return Ok(base_python);
    }

    let mut arguments: Vec<PathOrArg> = vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        base_python.clone().into(),
        "--system".into(),
        "--no-progress".into(),
        "--torch-backend=auto".into(),
    ];
    arguments.extend(SHARED_RUNTIME_PACKAGES.iter().map(|item| (*item).into()));
    run_logged_uv_command(takokit_root, &log, &uv, &arguments)?;
    std::fs::write(&marker, identity)?;
    Ok(base_python)
}

fn run_logged_uv_command(
    takokit_root: &Path,
    log: &Path,
    uv: &Path,
    arguments: &[PathOrArg],
) -> PackageResult<()> {
    let cache = takokit_root.join("cache").join("uv");
    let python = takokit_root.join("tools").join("python");
    let tools = takokit_root.join("tools").join("uv-tools");
    let bins = takokit_root.join("tools").join("bin");
    let cache = cache.to_string_lossy().into_owned();
    let python = python.to_string_lossy().into_owned();
    let tools = tools.to_string_lossy().into_owned();
    let bins = bins.to_string_lossy().into_owned();
    run_logged_command_with_env(
        log,
        uv,
        arguments,
        &[
            ("UV_CACHE_DIR", cache.as_str()),
            ("UV_PYTHON_INSTALL_DIR", python.as_str()),
            ("UV_TOOL_DIR", tools.as_str()),
            ("UV_TOOL_BIN_DIR", bins.as_str()),
            ("UV_PYTHON_PREFERENCE", "only-managed"),
        ],
    )
}

fn install_adapter_source(
    adapter_dir: &Path,
    log: &Path,
    source: &AdapterSourceSpec,
) -> PackageResult<PathBuf> {
    let destination = adapter_dir.join("source");
    let marker = destination.join(".takokit-revision");
    if destination.is_dir()
        && std::fs::read_to_string(&marker)
            .ok()
            .is_some_and(|revision| revision.trim() == source.revision)
    {
        return Ok(destination);
    }
    if destination.exists() {
        std::fs::remove_dir_all(&destination)?;
    }
    let temporary = adapter_dir.join("source.download");
    if temporary.exists() {
        std::fs::remove_dir_all(&temporary)?;
    }
    let clone_args = if source.recursive {
        vec![
            "clone".into(),
            "--recursive".into(),
            "--no-checkout".into(),
            source.repository.into(),
            temporary.clone().into(),
        ]
    } else {
        vec![
            "clone".into(),
            "--no-checkout".into(),
            source.repository.into(),
            temporary.clone().into(),
        ]
    };
    run_logged_command(log, "git", &clone_args)?;
    run_logged_command(
        log,
        "git",
        &[
            "-C".into(),
            temporary.clone().into(),
            "checkout".into(),
            "--detach".into(),
            source.revision.into(),
        ],
    )?;
    if source.recursive {
        run_logged_command(
            log,
            "git",
            &[
                "-C".into(),
                temporary.clone().into(),
                "submodule".into(),
                "update".into(),
                "--init".into(),
                "--recursive".into(),
            ],
        )?;
    }
    std::fs::write(temporary.join(".takokit-revision"), source.revision)?;
    std::fs::rename(&temporary, &destination)?;
    Ok(destination)
}

fn uv_pip_install(
    takokit_root: &Path,
    uv: &Path,
    python: &Path,
    log: &Path,
    dependencies: impl IntoIterator<Item = PathOrArg>,
) -> PackageResult<()> {
    let mut arguments: Vec<PathOrArg> = vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        python.to_path_buf().into(),
        "--no-progress".into(),
        "--torch-backend=auto".into(),
    ];
    arguments.extend(dependencies);
    run_logged_uv_command(takokit_root, log, uv, &arguments)
}

pub(crate) fn write_adapter_record(path: &Path, record: &AdapterRecord) -> PackageResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| PackageError::ArtifactInstallFailed {
            artifact: record.id.clone(),
            reason: "adapter manifest path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, toml::to_string_pretty(record)?)?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "takokit-runtime-python-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn shared_runtime_identity_tracks_python_abi() {
        assert_eq!(
            shared_runtime_identity("3.11"),
            "shared-python-v1-py3.11"
        );
        assert_ne!(
            shared_runtime_identity("3.10"),
            shared_runtime_identity("3.12")
        );
    }

    #[test]
    fn adapter_venv_must_enable_shared_packages() {
        let root = test_directory("site-packages");
        std::fs::create_dir_all(&root).expect("create test venv");
        std::fs::write(
            root.join("pyvenv.cfg"),
            "include-system-site-packages = true\n",
        )
        .expect("write pyvenv.cfg");
        assert!(venv_inherits_shared_packages(&root));

        std::fs::write(
            root.join("pyvenv.cfg"),
            "include-system-site-packages = false\n",
        )
        .expect("rewrite pyvenv.cfg");
        assert!(!venv_inherits_shared_packages(&root));
        std::fs::remove_dir_all(root).expect("remove test venv");
    }

    #[test]
    fn managed_base_must_belong_to_takokit() {
        let root = test_directory("managed-base");
        let managed_root = root.join("tools").join("python");
        let base = managed_root.join(if cfg!(windows) {
            "python.exe"
        } else {
            "python"
        });
        let venv = root.join("venv");
        std::fs::create_dir_all(&managed_root).expect("create managed root");
        std::fs::create_dir_all(&venv).expect("create venv");
        std::fs::write(&base, b"").expect("create fake interpreter");
        std::fs::write(
            venv.join("pyvenv.cfg"),
            format!("base-executable = {}\n", base.display()),
        )
        .expect("write pyvenv.cfg");

        assert_eq!(
            managed_base_python(&venv, &root).expect("resolve managed Python"),
            base
        );
        std::fs::remove_dir_all(root).expect("remove test tree");
    }
}
