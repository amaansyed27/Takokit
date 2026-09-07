# Security

Please report security vulnerabilities privately to the Takokit maintainers rather than opening a public issue with exploit details, credentials, private audio, or sensitive local paths.

## Security model

Takokit is local-first. Core inference is intended to run on the user's machine. Network access is explicit for actions such as registry synchronization, pinned model/runtime downloads, and signed update checks/downloads.

Any future feature that sends user content outside the machine must be visible and documented before release.

## Security-sensitive areas

Reports are especially useful for:

- voice cloning/conversion/training consent bypasses,
- hidden or unexpected network transmission,
- local API authentication/Host/Origin bypasses,
- unsafe runner/adapter process execution,
- arbitrary-command exposure through local GUI/control helpers,
- path traversal or unsafe symlink/hardlink extraction,
- archive/update/install replacement outside owned paths,
- foreign-process termination through server lifecycle bugs,
- release-signature or SHA-256 verification bypasses,
- model-source pin/integrity violations,
- destructive reset/uninstall confirmation bypasses,
- leaking audio, transcripts, reference voices, datasets, API tokens, or sensitive logs.

## Local API

Default loopback use is designed for zero-configuration local access. Non-loopback binding requires Bearer authentication and privileged control routes receive additional protections. A local route that opens a picker or path must stay narrowly scoped; it must not become a generic remote filesystem/command bridge.

## Model and managed-runtime trust

Takokit should not execute arbitrary scripts supplied by a model manifest. Supported model paths use declared runner/adapter contracts and pinned/verified dependencies where the runtime architecture requires them.

Managed tools such as uv are Takokit-owned: platform/architecture/version/integrity are validated and managed state must be safely replaceable/recoverable. System/Homebrew tools are not silently copied into Takokit's managed trust boundary.

## Release and update trust

Production Takokit release metadata is signed with the production release identity and artifacts are checksum-verified before installation/replacement. Test-fixture signing identities must never be accepted as stable production releases.

Apple Developer ID/notarization and Windows Authenticode are additional platform trust layers. Takokit must report their actual state and must not disable Gatekeeper or weaken operating-system security settings to work around missing signing credentials.

## Voice safety and sensitive data

Cloning, conversion, custom voice creation, and training require ownership or explicit permission for the relevant voice/reference/training material. Takokit stores potentially sensitive voices, sessions, transcripts, audio outputs, and logs locally; users should protect backups and shared project directories accordingly.

## Destructive operations

Normal uninstall preserves `~/.takokit` and project `.tako` data. Full-data reset/purge is separate and confirmation-gated. Security fixes must not weaken those path acknowledgements merely to simplify cleanup.
