use std::{env, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=TAKOKIT_BUILD_ID");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");

    let build_id = explicit_build_id()
        .or_else(git_build_id)
        .unwrap_or_else(|| {
            format!(
                "version-{}",
                env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into())
            )
        });

    println!("cargo:rustc-env=TAKOKIT_BUILD_ID={build_id}");

    #[cfg(windows)]
    embed_windows_branding();
}

#[cfg(windows)]
fn embed_windows_branding() {
    let icon = Path::new("../../assets/favicon/favicon.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(icon.to_string_lossy().as_ref())
        .set("ProductName", "Takokit")
        .set("FileDescription", "Takokit local voice AI runtime")
        .set("CompanyName", "Dawnlight Labs");
    resource
        .compile()
        .expect("compile Takokit Windows icon resource");
}

fn explicit_build_id() -> Option<String> {
    env::var("TAKOKIT_BUILD_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_build_id() -> Option<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let repository_root = Path::new(&manifest_dir).join("../..");
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
