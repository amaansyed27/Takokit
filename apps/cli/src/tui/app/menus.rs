pub const HOME_ACTIONS: [(&str, &str); 8] = [
    ("Speak", "Text → speech using a built-in or cloned voice"),
    ("Transcribe", "Audio → text with an installed speech model"),
    (
        "Create voice",
        "Instant reference clone or train a reusable local voice",
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
