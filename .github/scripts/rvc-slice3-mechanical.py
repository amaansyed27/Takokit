from pathlib import Path

for raw in [
    "apps/gui/src/features/voices/AdvancedVoiceStudio.tsx",
    "apps/gui/src/features/voices/RvcSamplesPanel.tsx",
    "apps/gui/src/features/voices/RvcStudioWorkspace.tsx",
    "apps/gui/src/features/voices/RvcTrainingPanel.tsx",
]:
    path = Path(raw)
    text = path.read_text(encoding="utf-8")
    text = text.replace('.replaceAll("_", " ")', '.replace(/_/g, " ")')
    text = text.replace('.replaceAll("-", " ")', '.replace(/-/g, " ")')
    path.write_text(text, encoding="utf-8")
