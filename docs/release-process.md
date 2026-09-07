# Release process

Takokit uses reviewed branches, exact-commit CI, signed release metadata, and deliberate tags. Public artifacts for one version must come from one accepted source commit.

## Maturity policy

- `0.x.x` — **Beta**. Public and updateable; compatibility/UX can still evolve before 1.0.
- `1.0.0` — first **Stable** release target and first stable compatibility commitment.

A beta label never justifies fake artifacts, unsupported-platform claims, or skipping integrity/security gates.

## Development flow

```text
latest main
  ↓
dedicated slice/feature branch
  ↓
implementation + deterministic tests
  ↓
exact-HEAD CI / distribution acceptance
  ↓
human checks that automation cannot prove
  ↓
PR → review → merge
  ↓
main regression
  ↓
deliberate release tag
  ↓
production release workflow
```

Do not create parallel replacement branches for ordinary fixes inside the same slice. Do not push implementation directly to `main` unless the repository owner explicitly approves an exceptional recovery workflow.

## v0.3.0 multi-platform release

The v0.3.0 workflow publishes from the `v0.3.0` tag only after its reusable cross-platform acceptance succeeds with production signing enabled.

Required published targets:

- Windows x86_64
- Linux x86_64
- macOS arm64

Experimental macOS x86_64 and unsupported Linux arm64 are not added to the stable platform set merely because source code can compile for them.

## Trust and integrity

Takokit's production release metadata uses signing identity:

```text
takokit-release-v1
```

The release set contains platform artifacts plus signed manifests/index, SHA-256 checksums, release notes, and build provenance. Platform code-signing/notarization is additional evidence:

- Windows Authenticode can be reported independently of the Takokit manifest signature.
- Apple Developer ID / notarization can be reported independently of the Takokit manifest signature.

Never substitute the deterministic test signing key for production stable metadata.

## Backwards compatibility

When release metadata evolves, existing public updaters remain part of the acceptance surface. In particular, v0.3.0 preserves the v0.2.0 Windows update discovery/manifest contract while adding the multi-platform index and Unix platform manifests.

Release acceptance should explicitly test supported upgrade paths instead of assuming a new installer proves updates work.

## Public install surfaces

Windows:

```powershell
irm https://takokit.dawnlightlabs.com/install.ps1 | iex
```

Linux/macOS:

```bash
curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh
```

The public bootstrap must resolve stable production metadata, select the correct OS/architecture artifact, verify integrity, install safely, and fail closed when the platform release is absent or metadata is invalid.

## Stable metadata and site deployment

Do not point the public site at a version/tag before that tag and its release assets exist. After release:

1. verify GitHub Release assets and production signing identity,
2. verify Windows/Linux/macOS stable metadata endpoints,
3. verify raw `install.ps1` / `install.sh`,
4. test a fresh install path for each published OS where practical,
5. verify a previously installed supported version discovers the update,
6. deploy/verify release-facing site copy,
7. close release tracking issues only after the published state is proven.

## Failure handling

If an exact-HEAD gate fails:

1. inspect the exact failing command/log,
2. decide whether production code or an obsolete test expectation is wrong,
3. fix on the same branch,
4. create a new commit,
5. rerun gates on the new exact HEAD.

Do not weaken process ownership, path/archive safety, release signature/checksum validation, authentication, or consent requirements merely to turn CI green.
