//! Removes exact dependency duplicates from managed adapter overlays.

use super::shared::run_logged_uv_command;
use crate::{
    runtime_command::{run_logged_command, PathOrArg},
    *,
};
use std::path::{Path, PathBuf};

const OVERLAY_PROBE: &str = r#"
import importlib.metadata as metadata
import json
import pathlib
import re
import subprocess
import sys
import sysconfig

base_python = sys.argv[1]
report_path = pathlib.Path(sys.argv[2])

probe = r'''
import importlib.metadata as metadata
import json
import re

def canonical(name):
    return re.sub(r"[-_.]+", "-", name).lower()

packages = {}
for distribution in metadata.distributions():
    name = distribution.metadata.get("Name")
    if name:
        packages[canonical(name)] = distribution.version
print(json.dumps(packages, sort_keys=True))
'''

completed = subprocess.run(
    [base_python, "-I", "-c", probe],
    check=True,
    capture_output=True,
    text=True,
    encoding="utf-8",
)
base_packages = json.loads(completed.stdout)

def canonical(name):
    return re.sub(r"[-_.]+", "-", name).lower()

local_site = pathlib.Path(sysconfig.get_path("purelib"))
protected = {"pip", "setuptools", "wheel"}
duplicates = set()
for distribution in metadata.distributions(path=[str(local_site)]):
    name = distribution.metadata.get("Name")
    if not name:
        continue
    normalized = canonical(name)
    if normalized not in protected and base_packages.get(normalized) == distribution.version:
        duplicates.add(normalized)

report_path.write_text(
    "".join(f"{name}\n" for name in sorted(duplicates)),
    encoding="utf-8",
)
"#;

pub(super) fn prune_shared_overlay(
    takokit_root: &Path,
    uv: &Path,
    adapter_python: &Path,
    shared_python: &Path,
    adapter_dir: &Path,
    log: &Path,
) -> PackageResult<usize> {
    let helper = adapter_dir.join(".takokit-overlay-probe.py");
    let plan = adapter_dir.join(".takokit-overlay-prune.txt");
    let inherited = adapter_dir.join(".takokit-inherited-packages.txt");
    std::fs::write(&helper, OVERLAY_PROBE)?;

    run_overlay_probe(log, adapter_python, shared_python, &helper, &plan)?;
    let packages = parse_prunable_packages(&std::fs::read_to_string(&plan)?)?;
    if !packages.is_empty() {
        let arguments = uninstall_arguments(adapter_python, &packages);
        run_logged_uv_command(takokit_root, log, uv, &arguments)?;

        run_overlay_probe(log, adapter_python, shared_python, &helper, &plan)?;
        let remaining = parse_prunable_packages(&std::fs::read_to_string(&plan)?)?;
        if !remaining.is_empty() {
            return Err(PackageError::ArtifactInstallFailed {
                artifact: "shared managed Python overlay".to_string(),
                reason: format!(
                    "UV left exact shared-package duplicates in the adapter: {}",
                    remaining.join(", ")
                ),
            });
        }
    }

    std::fs::write(
        &inherited,
        packages
            .iter()
            .map(|package| format!("{package}\n"))
            .collect::<String>(),
    )?;
    let _ = std::fs::remove_file(helper);
    let _ = std::fs::remove_file(plan);
    Ok(packages.len())
}

fn run_overlay_probe(
    log: &Path,
    adapter_python: &Path,
    shared_python: &Path,
    helper: &Path,
    report: &Path,
) -> PackageResult<()> {
    run_logged_command(
        log,
        adapter_python,
        &[
            "-I".into(),
            helper.to_path_buf().into(),
            shared_python.to_path_buf().into(),
            report.to_path_buf().into(),
        ],
    )
}

fn uninstall_arguments(python: &Path, packages: &[String]) -> Vec<PathOrArg> {
    let mut arguments: Vec<PathOrArg> = vec![
        "pip".into(),
        "uninstall".into(),
        "--python".into(),
        python.to_path_buf().into(),
    ];
    arguments.extend(packages.iter().map(|package| package.as_str().into()));
    arguments
}

fn parse_prunable_packages(source: &str) -> PackageResult<Vec<String>> {
    let mut packages = source
        .lines()
        .map(str::trim)
        .filter(|package| !package.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if let Some(invalid) = packages.iter().find(|package| {
        package
            .bytes()
            .next()
            .is_none_or(|byte| !byte.is_ascii_alphanumeric())
            || !package
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }) {
        return Err(PackageError::ArtifactInstallFailed {
            artifact: "shared managed Python overlay".to_string(),
            reason: format!("dependency probe returned an invalid package name: {invalid}"),
        });
    }
    packages.sort();
    packages.dedup();
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_report_is_validated_sorted_and_deduplicated() {
        assert_eq!(
            parse_prunable_packages("torch\nnumpy\ntorch\n").expect("parse report"),
            vec!["numpy".to_string(), "torch".to_string()]
        );
        assert!(parse_prunable_packages("--system\n").is_err());
    }

    #[test]
    fn uninstall_targets_only_the_adapter_interpreter() {
        let arguments = uninstall_arguments(
            Path::new("adapter-python"),
            &["numpy".to_string(), "torch".to_string()],
        );
        let arguments = arguments
            .iter()
            .map(|argument| argument.as_os_str().to_string_lossy())
            .collect::<Vec<_>>();
        assert_eq!(
            arguments,
            [
                "pip",
                "uninstall",
                "--python",
                "adapter-python",
                "numpy",
                "torch"
            ]
        );
    }
}
