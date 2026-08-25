from pathlib import Path


def edit(path, old, new, count=1):
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"missing expected block in {path}: {old!r}")
    p.write_text(text.replace(old, new, count), encoding="utf-8")

# Wire focused CLI/API regression modules that were added as separate files.
edit("apps/cli/src/tests.rs", "use super::*;\n", "use super::*;\n\nmod rvc;\n")

handler = Path("crates/takokit-server/src/handlers/rvc_voices.rs")
text = handler.read_text(encoding="utf-8")
if "mod tests;" not in text:
    text = text.rstrip() + "\n\n#[cfg(test)]\nmod tests;\n"
handler.write_text(text, encoding="utf-8")

# Clear the final compiler warnings without weakening any validation.
edit(
    "apps/cli/src/lib.rs",
    "resolve_execution_plan, runner_runtime_layout, voice_contract_for_model, InstallModelOptions,",
    "resolve_execution_plan, runner_runtime_layout, InstallModelOptions,",
)
edit(
    "apps/cli/src/tui/input/advanced_rvc.rs",
    "#[cfg(test)]\nmod tests {\n    use super::*;\n\n",
    "#[cfg(test)]\nmod tests {\n",
)
edit(
    "crates/takokit-server/src/native_picker.rs",
    "impl PickerKind {\n    fn windows_filter(self) -> &'static str {",
    "impl PickerKind {\n    #[cfg(any(windows, test))]\n    fn windows_filter(self) -> &'static str {",
)
