#!/usr/bin/env python3
"""One-time deterministic Slice 2 module split used before merge."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, content: str) -> None:
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")


def require_once(source: str, marker: str, path: str) -> None:
    count = source.count(marker)
    if count != 1:
        raise RuntimeError(f"expected one marker in {path}: {marker!r}; found {count}")


def split_convert_page() -> None:
    path = "apps/gui/src/features/convert/ConvertPage.tsx"
    source = read(path)
    marker = "\ntype ConversionPathCardProps = {"
    require_once(source, marker, path)
    head, tail = source.split(marker, 1)
    block = "type ConversionPathCardProps = {" + tail
    block = block.replace("function ConversionPathCard(", "export function ConversionPathCard(", 1)
    block = block.replace("function NumberField(", "export function NumberField(", 1)
    block = block.replace("function ReviewItem(", "export function ReviewItem(", 1)
    block = block.replace("function displayFileName(", "export function displayFileName(", 1)
    block = block.replace("function formatBytes(", "export function formatBytes(", 1)
    components = (
        'import { Check, FileAudio, FolderOpen, X } from "lucide-react";\n'
        'import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";\n'
        'import { ProductButton } from "../../components/ui/ProductButton";\n\n'
        + block
    )
    head = head.replace("  FolderOpen,\n", "")
    import_marker = 'import { consumeCloneIntent } from "../../lib/workflowIntent";'
    require_once(head, import_marker, path)
    head = head.replace(
        import_marker,
        import_marker
        + '\nimport { ConversionPathCard, NumberField, ReviewItem, displayFileName, formatBytes } from "./ConvertComponents";',
        1,
    )
    write(path, head.rstrip() + "\n")
    write("apps/gui/src/features/convert/ConvertComponents.tsx", components)


def split_app_state_access() -> None:
    path = "apps/cli/src/tui/app.rs"
    source = read(path)
    start = "    pub fn request_confirmation"
    end = "    pub fn set_status"
    require_once(source, start, path)
    require_once(source, end, path)
    before, rest = source.split(start, 1)
    moved, after = rest.split(end, 1)
    moved = start + moved
    write("apps/cli/src/tui/app/state_access.rs", "use super::*;\n\nimpl App {\n" + moved + "}\n")
    source = before + end + after
    insert = "\n\npub const HOME_ACTIONS"
    require_once(source, insert, path)
    source = source.replace(insert, "\n\nmod state_access;\n\npub const HOME_ACTIONS", 1)
    write(path, source)


def split_convert_submit() -> None:
    path = "apps/cli/src/tui/input/forms.rs"
    source = read(path)
    start = "pub(super) fn submit_convert"
    end = "#[cfg(test)]"
    require_once(source, start, path)
    require_once(source, end, path)
    before, rest = source.split(start, 1)
    moved, after = rest.split(end, 1)
    moved = start + moved
    write("apps/cli/src/tui/input/forms/convert_submit.rs", "use super::*;\n\n" + moved.rstrip() + "\n")
    source = before + end + after
    insert = "use super::{normalize_path_field, picker};"
    require_once(source, insert, path)
    source = source.replace(
        insert,
        insert + "\n\nmod convert_submit;\nuse convert_submit::submit_convert;",
        1,
    )
    write(path, source)


def split_daemon_runtime() -> None:
    path = "apps/cli/src/daemon.rs"
    source = read(path)
    start = "fn verify_identity"
    end = "fn now()"
    require_once(source, start, path)
    require_once(source, end, path)
    before, rest = source.split(start, 1)
    moved, after = rest.split(end, 1)
    moved = start + moved
    private_names = [
        "verify_identity",
        "startup_lock",
        "daemon_lock_is_held",
        "port_is_occupied",
        "takokit_health_responds",
        "log_path",
        "managed_daemon_executable",
        "preferred_daemon_executable",
        "canonical_exe",
        "canonical_root",
    ]
    for name in private_names:
        moved = moved.replace(f"fn {name}(", f"pub(super) fn {name}(", 1)
    write("apps/cli/src/daemon/runtime.rs", "use super::*;\n\n" + moved.rstrip() + "\n")
    imports = """mod runtime;
pub use runtime::write_atomic;
pub(crate) use runtime::build_freshness;
use runtime::{
    canonical_exe, canonical_root, daemon_lock_is_held, log_path, managed_daemon_executable,
    port_is_occupied, preferred_daemon_executable, startup_lock, takokit_health_responds,
    verify_identity,
};

"""
    source = before + end + after
    marker = "const IDENTITY_WAIT"
    require_once(source, marker, path)
    source = source.replace(marker, imports + marker, 1)
    write(path, source)


def split_api_tests() -> None:
    path = "crates/takokit-core/src/api.rs"
    source = read(path)
    marker = "#[cfg(test)]\nmod tests {\n"
    require_once(source, marker, path)
    before, body = source.split(marker, 1)
    if not body.rstrip().endswith("}"):
        raise RuntimeError("api test module does not end with a closing brace")
    body = body.rstrip()
    body = body[:-1].rstrip() + "\n"
    write("crates/takokit-core/src/api/tests.rs", body)
    write(path, before.rstrip() + "\n\n#[cfg(test)]\nmod tests;\n")


def split_ui_form_helpers() -> None:
    path = "apps/cli/src/tui/ui/forms.rs"
    source = read(path)
    marker = "fn render_convert_value("
    require_once(source, marker, path)
    before, moved = source.split(marker, 1)
    moved = marker + moved
    moved = moved.replace("fn render_convert_value(", "pub(super) fn render_convert_value(", 1)
    moved = moved.replace("fn render_intro(", "pub(super) fn render_intro(", 1)
    write("apps/cli/src/tui/ui/forms/helpers.rs", "use super::*;\n\n" + moved)
    insert = "use crate::tui::{"
    require_once(before, insert, path)
    before = before.replace(
        insert,
        "mod helpers;\nuse helpers::{render_convert_value, render_intro};\n\n" + insert,
        1,
    )
    write(path, before.rstrip() + "\n")


def main() -> None:
    split_convert_page()
    split_app_state_access()
    split_convert_submit()
    split_daemon_runtime()
    split_api_tests()
    split_ui_form_helpers()
    print("Slice 2 module split complete")


if __name__ == "__main__":
    main()
