export const RELEASE_DOCS = {
  "beta-status": {
    title: "Beta status",
    intro: "Takokit remains Beta throughout the 0.x.y series. Release numbering and promotion to Stable are explicit maintainer decisions rather than automatic consequences of shipping a release.",
    sections: [
      {
        id: "policy",
        title: "Version policy",
        code: `0.x.y   Beta — public, usable, updateable, but interfaces and platform behavior can still evolve
Default beta release bump: y = y + 1 (for example 0.3.0 → 0.3.1)
Change x only when the maintainer explicitly requests it
Stable is published only when the maintainer explicitly declares the Stable transition; do not infer it from a routine release`,
      },
      {
        id: "beta-expectations",
        title: "What Beta means",
        items: ["Real signed release artifacts and update metadata", "Evidence-based platform/model support labels", "Breaking changes remain possible before Stable when required to fix architecture or safety", "Release notes should call out migrations and changed behavior", "Bugs and rough edges are expected to be reported rather than hidden"],
      },
      {
        id: "model-evidence",
        title: "Model support is separate from platform support",
        body: ["A working Windows/Linux/macOS package does not prove that every GPU-heavy model runs on every operating system. Model pages and platform support docs distinguish supported, tested, experimental, hardware-dependent, and unsupported paths."],
      },
    ],
  },
  "platform-support": {
    title: "Platform support",
    intro: "The v0.3.0 beta release target is Windows x86_64, Linux x86_64, and macOS Apple Silicon. Unsupported/experimental architectures are not silently presented as stable downloads.",
    sections: [
      {
        id: "matrix",
        title: "v0.3.0 target matrix",
        code: `Windows 10/11 x86_64   Supported beta   Inno installer + portable ZIP
Linux x86_64           Supported beta   install.sh + portable tar.gz + desktop launcher
macOS 12+ arm64        Supported beta   install.sh + tar.gz + Takokit.app
macOS x86_64           Experimental     No stable v0.3.0 download promise
Linux arm64            Unsupported      Not published in v0.3.0`,
      },
      {
        id: "accelerators",
        title: "Accelerators",
        body: ["CPU is sufficient for the core packaged runtime and representative small-model acceptance. CUDA/MPS/other accelerator compatibility is model- and upstream-dependent. Takokit does not infer MPS parity from CUDA support."],
      },
      {
        id: "mac-signing",
        title: "macOS signing and Gatekeeper",
        body: ["The release reports its actual Apple signing/notarization state. If a beta build is only ad-hoc signed, macOS may apply Gatekeeper restrictions to browser-downloaded artifacts."],
        warning: "Takokit never disables Gatekeeper, removes quarantine automatically to bypass macOS policy, or asks users to weaken system security as part of normal installation.",
      },
    ],
  },
  "release-channels": {
    title: "Release channels",
    intro: "Takokit verifies signed release manifests before downloading/installing updates. Stable and preview channels are explicit user-visible choices.",
    sections: [
      {
        id: "status",
        title: "Inspect update state",
        commands: ["tako update status", "tako update check"],
      },
      {
        id: "channels",
        title: "Choose a channel",
        commands: ["tako update channel stable", "tako update channel preview"],
        body: ["Stable points at approved public releases. Preview is for deliberately published preview metadata; test-fixture signing identities are not accepted as stable production releases."],
      },
      {
        id: "automation",
        title: "Automatic checks",
        commands: ["tako update configure --automatic-checks on --automatic-download off"],
        body: ["Automatic manifest checks are conservative. Verified background download can be enabled separately, but update installation remains an explicit action."],
      },
      {
        id: "apply",
        title: "Download and apply",
        commands: ["tako update download", "tako update apply"],
        body: ["The updater verifies the signed manifest, artifact SHA-256, platform/architecture, stages replacement, preserves mutable user/workspace state, and uses rollback/recovery contracts when replacement fails."],
      },
      {
        id: "trust",
        title: "Release trust",
        body: ["Production multi-platform manifests use the Takokit release signing identity `takokit-release-v1`. Apple code signing and Windows Authenticode, when present, are additional platform trust layers rather than replacements for Takokit's release-manifest verification."],
      },
    ],
  },
  "privacy-safety": {
    title: "Privacy and voice safety",
    intro: "Takokit is designed as a local voice runtime. Local processing, explicit downloads, and consent-backed voice workflows are separate boundaries that users should understand.",
    sections: [
      {
        id: "local",
        title: "Local processing",
        body: ["Inference through the local Takokit runtime stays on the machine unless a selected external model/runtime explicitly requires otherwise. Takokit does not need a hosted inference API for its core local workflows."],
      },
      {
        id: "network",
        title: "When Takokit uses the network",
        items: ["Registry synchronization", "Model/runtime downloads from pinned upstream sources", "Signed update checks/downloads", "User-selected non-loopback API access"],
        body: ["Network downloads can reveal normal request metadata to the hosting provider. Audio/text content is not uploaded merely because Takokit checks for a release or model artifact."],
      },
      {
        id: "api",
        title: "Local API security",
        body: ["Loopback use is intentionally zero-configuration. Non-loopback binding requires a Bearer token and Takokit applies Host/Origin protections to privileged local-control routes."],
      },
      {
        id: "voice-consent",
        title: "Voice consent",
        body: ["Cloning, conversion, custom voice creation, and RVC training require ownership or explicit permission for the relevant voice/reference/training material."],
        warning: "Do not use Takokit to create or distribute an impersonation model of a person whose voice you do not own or have permission to use.",
      },
      {
        id: "sensitive-data",
        title: "Sensitive local data",
        body: ["Reference recordings, voice profiles, transcripts, generated audio, sessions, and logs may be sensitive. Protect backups and shared workspaces accordingly; use normal uninstall versus destructive reset intentionally."],
      },
      {
        id: "telemetry",
        title: "Telemetry",
        body: ["Release-facing documentation does not claim hidden analytics or remote inference. If telemetry is introduced in a future beta, its collection, purpose, and controls must be documented before release."],
      },
    ],
  },
};
