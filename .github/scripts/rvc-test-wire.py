from pathlib import Path


def edit(path, old, new, count=1):
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"missing expected block in {path}: {old!r}")
    p.write_text(text.replace(old, new, count), encoding="utf-8")

edit("apps/cli/src/tests.rs", "use super::*;\n", "use super::*;\n\nmod rvc;\n")

handler = Path("crates/takokit-server/src/handlers/rvc_voices.rs")
text = handler.read_text(encoding="utf-8")
if "mod tests;" not in text:
    text = text.rstrip() + "\n\n#[cfg(test)]\nmod tests;\n"
handler.write_text(text, encoding="utf-8")
