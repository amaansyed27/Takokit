from pathlib import Path


def edit(path, old, new, count=1):
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"missing expected block in {path}: {old[:100]!r}")
    p.write_text(text.replace(old, new, count), encoding="utf-8")

# Keep TUI app state under the repository hard line limit.
app = "apps/cli/src/tui/app.rs"
start = '''pub const HOME_ACTIONS: [(&str, &str); 8] = [
    ("Speak", "Text → speech using a built-in or cloned voice"),
    ("Transcribe", "Audio → text with an installed speech model"),
    (
        "Create voice",
        "Instant Clone or persistent Advanced RVC Voice Studio",
    ),
    (
        "Convert voice",
        "Audio → another voice while keeping the original words",
    ),
    ("Manage", "Inspect models, runners, and the local service"),
    ("Sessions", "Open prior work or start a clean session"),
    (
        "Workspace",
        "View or change the project-specific .tako location",
    ),
    (
        "Activity",
        "Review the latest result, output path, and next action",
    ),
];

pub const MANAGE_ACTIONS: [(&str, &str); 4] = [
    ("Installed models", "Use, repair, or remove local models"),
    (
        "Model library",
        "Browse and pull models from the Takokit registry",
    ),
    ("Runners", "Inspect and repair shared execution runtimes"),
    ("System", "Daemon status, diagnostics, logs, and GUI"),
];

'''
edit(app, start, "")
edit(app, "mod state_access;\n", "mod menus;\nmod state_access;\n\npub use menus::{HOME_ACTIONS, MANAGE_ACTIONS};\n")

# Move static Convert constants out of the page component.
components = "apps/gui/src/features/convert/ConvertComponents.tsx"
p = Path(components)
text = p.read_text(encoding="utf-8")
text += '''\nexport const f0Options = [\n  { value: \"rmvpe\", label: \"RMVPE\" },\n  { value: \"harvest\", label: \"Harvest\" },\n  { value: \"crepe\", label: \"CREPE\" },\n  { value: \"pm\", label: \"Parselmouth\" }\n];\n\nexport const emptyReview = {\n  words: false,\n  timbre: false,\n  similarity: false,\n  artifacts: false\n};\n'''
p.write_text(text, encoding="utf-8")

convert = "apps/gui/src/features/convert/ConvertPage.tsx"
edit(convert,
     'import { ConversionPathCard, NumberField, ReviewItem, displayFileName, formatBytes } from "./ConvertComponents";',
     'import { ConversionPathCard, NumberField, ReviewItem, displayFileName, emptyReview, f0Options, formatBytes } from "./ConvertComponents";')
edit(convert, '''type ReviewState = {
  words: boolean;
  timbre: boolean;
  similarity: boolean;
  artifacts: boolean;
};

const f0Options = [
  { value: "rmvpe", label: "RMVPE" },
  { value: "harvest", label: "Harvest" },
  { value: "crepe", label: "CREPE" },
  { value: "pm", label: "Parselmouth" }
];

const emptyReview: ReviewState = {
  words: false,
  timbre: false,
  similarity: false,
  artifacts: false
};

''', "type ReviewState = typeof emptyReview;\n\n")
