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

# RVC export is positional, but its internal clap ID must not collide with the
# global --output OutputFormat option.
edit(
    "apps/cli/src/args/rvc.rs",
    "    Export {\n        voice: String,\n        output: PathBuf,",
    "    Export {\n        voice: String,\n        package: PathBuf,",
)
edit(
    "apps/cli/src/rvc_voice_command.rs",
    "        RvcVoiceCommand::Export {\n            voice,\n            output,\n            sign,",
    "        RvcVoiceCommand::Export {\n            voice,\n            package,\n            sign,",
)
edit(
    "apps/cli/src/rvc_voice_command.rs",
    '                "output":output,',
    '                "output":package,',
)
edit(
    "apps/cli/src/tests/rvc.rs",
    "        RvcVoiceCommand::Export { voice, output, sign: true, .. }\n            if voice == \"voice-id\" && output == PathBuf::from(r\"C:\\\\exports\\\\voice ü.takovoice\")",
    "        RvcVoiceCommand::Export { voice, package, sign: true, .. }\n            if voice == \"voice-id\" && package == PathBuf::from(r\"C:\\\\exports\\\\voice ü.takovoice\")",
)

# Clear Slice 3 compiler warnings without weakening any validation.
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
    "apps/cli/src/tui/advanced_rvc.rs",
    "\n    pub fn clear_paths(&mut self) {\n        self.path.clear();\n        self.path_cursor = 0;\n        self.index.clear();\n        self.index_cursor = 0;\n    }\n",
    "\n",
)
edit(
    "crates/takokit-server/src/native_picker.rs",
    "impl PickerKind {\n    fn windows_filter(self) -> &'static str {",
    "impl PickerKind {\n    #[cfg(any(windows, test))]\n    fn windows_filter(self) -> &'static str {",
)
