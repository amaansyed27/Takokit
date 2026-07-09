use std::{
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Serialize;
use takokit_core::RuntimeConfig;
use takokit_models::ModelRegistry;
use takokit_package::{
    current_platform_id, runner_runtime_layout, InstalledRegistry, PackageRegistry,
    RunnerLifecycleState,
};
use takokit_store::LocalStore;

use crate::gui;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorCheck {
    section: &'static str,
    status: CheckStatus,
    label: String,
    detail: Option<String>,
}

impl DoctorCheck {
    #[cfg(test)]
    pub fn is_ok(&self) -> bool {
        self.status == CheckStatus::Ok
    }

    #[cfg(test)]
    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct DoctorReport {
    checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn checks(&self) -> &[DoctorCheck] {
        &self.checks
    }

    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == CheckStatus::Fail)
    }

    fn push(&mut self, check: DoctorCheck) {
        self.checks.push(check);
    }
}

pub fn run_doctor(
    config: &RuntimeConfig,
    store: &LocalStore,
    package_registry: &PackageRegistry,
    installed_registry: &InstalledRegistry,
) -> DoctorReport {
    let mut report = DoctorReport::default();

    check_path(&mut report, "Storage", "root", store.root(), true);
    for (label, path) in [
        ("models", store.models_dir()),
        ("runners", store.runners_dir()),
        ("blobs", store.blobs_dir()),
        ("manifests/models", store.model_manifests_dir()),
        ("manifests/runners", store.runner_manifests_dir()),
        (
            "manifests/installed-models",
            store.installed_model_records_dir(),
        ),
        (
            "manifests/installed-runners",
            store.installed_runner_records_dir(),
        ),
        ("voices", store.voices_dir()),
        ("datasets", store.datasets_dir()),
        ("outputs", store.outputs_dir()),
        ("cache", store.cache_dir()),
        ("logs", store.logs_dir()),
    ] {
        check_path(&mut report, "Storage", label, &path, true);
    }
    for (label, path) in [
        ("runners/python-managed", store.python_managed_runner_dir()),
        (
            "runners/python-managed/runtime",
            store.python_managed_runtime_dir(),
        ),
        ("runners/python-managed/env", store.python_managed_env_dir()),
        (
            "runners/python-managed/packages",
            store.python_managed_packages_dir(),
        ),
        (
            "runners/python-managed/wheels",
            store.python_managed_wheels_dir(),
        ),
        (
            "runners/python-managed/logs",
            store.python_managed_logs_dir(),
        ),
        (
            "runners/python-managed/manifests",
            store.python_managed_manifests_dir(),
        ),
        (
            "runners/python-managed/cache",
            store.python_managed_cache_dir(),
        ),
        (
            "runners/python-managed/adapters",
            store.python_managed_adapters_dir(),
        ),
    ] {
        check_optional_path(&mut report, "Managed runners", label, &path, true);
    }
    check_file(&mut report, "Storage", "config.toml", store.config_path());

    check_path(
        &mut report,
        "Registry",
        "local registry",
        package_registry.root(),
        true,
    );
    match package_registry.models() {
        Ok(models) if models.is_empty() => report.push(warn(
            "Registry",
            "0 model manifests found",
            "local mock registry is empty",
        )),
        Ok(models) => report.push(ok(
            "Registry",
            format!("{} model manifests found", models.len()),
        )),
        Err(error) => report.push(fail("Registry", "model manifests parse", error.to_string())),
    }
    match package_registry.runners() {
        Ok(runners) if runners.is_empty() => report.push(warn(
            "Registry",
            "0 runner manifests found",
            "local mock registry is empty",
        )),
        Ok(runners) => report.push(ok(
            "Registry",
            format!("{} runner manifests found", runners.len()),
        )),
        Err(error) => report.push(fail(
            "Registry",
            "runner manifests parse",
            error.to_string(),
        )),
    }

    match installed_registry.installed_model_records() {
        Ok(records) => report.push(ok(
            "Installed",
            format!("installed model records parse: {}", records.len()),
        )),
        Err(error) => report.push(fail(
            "Installed",
            "installed model records parse",
            error.to_string(),
        )),
    }
    match installed_registry.installed_runner_records() {
        Ok(records) => report.push(ok(
            "Installed",
            format!("installed runner records parse: {}", records.len()),
        )),
        Err(error) => report.push(fail(
            "Installed",
            "installed runner records parse",
            error.to_string(),
        )),
    }

    match installed_registry.installed_runner_record("takokit-python-managed") {
        Ok(record)
            if matches!(
                record.status,
                RunnerLifecycleState::RuntimeInstalled | RunnerLifecycleState::Ready
            ) =>
        {
            report.push(ok(
                "Managed runners",
                format!("python-managed runtime state: {:?}", record.status),
            ));
        }
        Ok(record) => report.push(warn(
            "Managed runners",
            format!("python-managed runtime state: {:?}", record.status),
            record.note,
        )),
        Err(_) => report.push(warn(
            "Managed runners",
            "python-managed runtime not initialized",
            "run: takokit runner pull takokit-python-managed && takokit runner install takokit-python-managed",
        )),
    }
    for runner_id in [
        "takokit-whispercpp",
        "takokit-onnx",
        "takokit-python-managed",
    ] {
        match package_registry.runner(runner_id) {
            Ok(manifest) => {
                let layout = runner_runtime_layout(store.root(), &manifest);
                match installed_registry.installed_runner_record(runner_id) {
                    Ok(record) => report.push(runner_state_check(
                        runner_id,
                        record.status,
                        format!("{}; logs: {}", record.note, pretty_path(&layout.logs)),
                    )),
                    Err(_) => report.push(warn(
                        "Runners",
                        format!("{runner_id} runtime missing"),
                        format!(
                            "run: takokit runner pull {runner_id} && takokit runner install {runner_id}"
                        ),
                    )),
                }
            }
            Err(error) => report.push(fail(
                "Runners",
                format!("{runner_id} manifest"),
                error.to_string(),
            )),
        }
    }

    if server_is_available(config) {
        report.push(ok(
            "Server",
            format!("available at {}", config.local_base_url()),
        ));
    } else {
        report.push(warn(
            "Server",
            "not running",
            format!(
                "start with takokit serve or takokit gui at {}",
                config.local_base_url()
            ),
        ));
    }

    let dist = gui::gui_dist_path();
    if dist.join("index.html").is_file() {
        report.push(ok("GUI", format!("dist found: {}", pretty_path(&dist))));
    } else {
        report.push(warn(
            "GUI",
            "GUI build missing",
            "Run: cd apps/gui && npm run build",
        ));
    }

    let registry = ModelRegistry::default();
    if registry.models().iter().any(|model| model.id == "mock-tts") {
        report.push(ok("Execution", "mock-tts available"));
    } else {
        report.push(fail(
            "Execution",
            "mock-tts availability",
            "built-in mock model is missing",
        ));
    }
    match installed_registry.installed_runner_records() {
        Ok(records)
            if records
                .iter()
                .any(|record| record.status == RunnerLifecycleState::Ready) =>
        {
            report.push(ok("Execution", "at least one real runner is ready"));
        }
        Ok(_) => report.push(warn(
            "Execution",
            "no real runner is ready",
            "install a runtime with takokit runner install <runner>",
        )),
        Err(error) => report.push(fail(
            "Execution",
            "runner runtime records",
            error.to_string(),
        )),
    }
    report.push(ok(
        "Platform",
        format!("platform: {}", current_platform_id()),
    ));

    report
}

pub fn print_report(report: &DoctorReport) {
    println!("Takokit Doctor");
    let mut current_section = "";
    for check in report.checks() {
        if check.section != current_section {
            current_section = check.section;
            println!();
            println!("{current_section}");
        }

        match &check.detail {
            Some(detail) => println!(
                "  {} {}: {}",
                status_marker(check.status),
                check.label,
                detail
            ),
            None => println!("  {} {}", status_marker(check.status), check.label),
        }
    }
}

fn check_path(
    report: &mut DoctorReport,
    section: &'static str,
    label: impl Into<String>,
    path: &Path,
    should_be_dir: bool,
) {
    let label = label.into();
    let exists = if should_be_dir {
        path.is_dir()
    } else {
        path.exists()
    };
    if exists {
        report.push(ok(section, format!("{label}: {}", pretty_path(path))));
    } else {
        report.push(fail(
            section,
            label,
            format!("missing at {}", pretty_path(path)),
        ));
    }
}

fn check_optional_path(
    report: &mut DoctorReport,
    section: &'static str,
    label: impl Into<String>,
    path: &Path,
    should_be_dir: bool,
) {
    let label = label.into();
    let exists = if should_be_dir {
        path.is_dir()
    } else {
        path.exists()
    };
    if exists {
        report.push(ok(section, format!("{label}: {}", pretty_path(path))));
    } else {
        report.push(warn(
            section,
            label,
            format!(
                "not initialized at {}; run takokit runner install takokit-python-managed",
                pretty_path(path)
            ),
        ));
    }
}

fn check_file(
    report: &mut DoctorReport,
    section: &'static str,
    label: &'static str,
    path: PathBuf,
) {
    if path.is_file() {
        report.push(ok(section, format!("{label}: {}", pretty_path(&path))));
    } else {
        report.push(fail(
            section,
            label,
            format!("missing at {}", pretty_path(&path)),
        ));
    }
}

fn pretty_path(path: &Path) -> String {
    let display = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string();

    display
        .strip_prefix(r"\\?\")
        .unwrap_or(&display)
        .to_string()
}

fn server_is_available(config: &RuntimeConfig) -> bool {
    let Ok(mut addrs) = config.bind_addr().to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };

    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

fn ok(section: &'static str, label: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        section,
        status: CheckStatus::Ok,
        label: label.into(),
        detail: None,
    }
}

fn warn(section: &'static str, label: impl Into<String>, detail: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        section,
        status: CheckStatus::Warn,
        label: label.into(),
        detail: Some(detail.into()),
    }
}

fn fail(section: &'static str, label: impl Into<String>, detail: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        section,
        status: CheckStatus::Fail,
        label: label.into(),
        detail: Some(detail.into()),
    }
}

fn runner_state_check(
    runner_id: &'static str,
    state: RunnerLifecycleState,
    detail: impl Into<String>,
) -> DoctorCheck {
    match state {
        RunnerLifecycleState::Ready => ok("Runners", format!("{runner_id} ready")),
        RunnerLifecycleState::Failed => fail("Runners", format!("{runner_id} failed"), detail),
        RunnerLifecycleState::RuntimeInstalled | RunnerLifecycleState::ContractInstalled => {
            warn("Runners", format!("{runner_id} state: {state}"), detail)
        }
        RunnerLifecycleState::RuntimeMissing => warn(
            "Runners",
            format!("{runner_id} runtime missing"),
            "run takokit runner install <runner>",
        ),
    }
}

fn status_marker(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "[ok]",
        CheckStatus::Warn => "[warn]",
        CheckStatus::Fail => "[fail]",
    }
}
