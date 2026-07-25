//! Shared Takokit-owned Python bases inherited by adapter environments.

use crate::{
    runtime_command::{run_logged_command_with_env, PathOrArg},
    runtime_uv::bootstrap_uv,
    *,
};
use std::path::{Path, PathBuf};

const SHARED_RUNTIME_VERSION: &str = "shared-python-v2";
const SHARED_RUNTIME_PACKAGES: &[&str] = &[
    "torch",
    "torchaudio",
    "huggingface_hub[hf_xet]",
    "numpy",
    "scipy",
    "soundfile",
    "packaging",
];

pub(super) fn shared_runtime_identity(python: &str) -> String {
    format!("{SHARED_RUNTIME_VERSION}-py{python}")
}

fn shared_runtime_dir(layout: &PythonManagedRunnerLayout, python: &str) -> PathBuf {
    layout
        .env
        .join(format!("shared-python-{}", python.replace('.', "_")))
}

pub(super) fn venv_inherits_shared_packages(venv: &Path) -> bool {
    std::fs::read_to_string(venv.join("pyvenv.cfg")).is_ok_and(|config| {
        config.lines().any(|line| {
            let normalized = line.replace(' ', "").to_ascii_lowercase();
            normalized == "include-system-site-packages=true"
        })
    })
}

fn managed_base_python(venv: &Path, takokit_root: &Path) -> PackageResult<PathBuf> {
    let config_path = venv.join("pyvenv.cfg");
    let config = std::fs::read_to_string(&config_path).map_err(|error| {
        PackageError::ArtifactInstallFailed {
            artifact: "shared managed Python".to_string(),
            reason: format!("could not read {}: {error}", config_path.display()),
        }
    })?;

    let value = |key: &str| {
        config.lines().find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            candidate
                .trim()
                .eq_ignore_ascii_case(key)
                .then(|| value.trim())
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

fn shared_package_install_arguments(base_python: &Path) -> Vec<PathOrArg> {
    let mut arguments: Vec<PathOrArg> = vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        base_python.to_path_buf().into(),
        "--system".into(),
        "--break-system-packages".into(),
        "--no-progress".into(),
        "--torch-backend=auto".into(),
    ];
    arguments.extend(SHARED_RUNTIME_PACKAGES.iter().map(|item| (*item).into()));
    arguments
}

pub(super) fn ensure_shared_python_runtime(
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

    let arguments = shared_package_install_arguments(&base_python);
    run_logged_uv_command(takokit_root, &log, &uv, &arguments)?;
    std::fs::write(&marker, identity)?;
    Ok(base_python)
}

pub(super) fn run_logged_uv_command(
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
        assert_eq!(shared_runtime_identity("3.11"), "shared-python-v2-py3.11");
        assert_ne!(
            shared_runtime_identity("3.10"),
            shared_runtime_identity("3.12")
        );
    }

    #[test]
    fn shared_install_explicitly_allows_the_verified_managed_base() {
        let arguments = shared_package_install_arguments(Path::new("managed-python"));
        let arguments = arguments
            .iter()
            .map(|argument| argument.as_os_str().to_string_lossy())
            .collect::<Vec<_>>();
        assert!(arguments.iter().any(|argument| argument == "--system"));
        assert!(arguments
            .iter()
            .any(|argument| argument == "--break-system-packages"));
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
