# Takokit registry protocol

Takokit's public registry is a small, auditable control plane. It maps human-friendly model references to pinned Takokit manifests while model bytes continue to flow directly from their declared upstream source.

## References

```text
[namespace/]model[:tag][@sha256:digest]
```

Examples:

```text
kokoro
whisper:small
library/qwen3-tts:0.6b-base
openvoice:2@sha256:<manifest-digest>
```

The default namespace is `library`. Omitting the tag selects the family's `default_tag`. The `latest` alias also selects that published default unless a family explicitly publishes a literal `latest` release.

A default is a Takokit-tested recommendation, not a floating upstream branch. Every release contains a complete pinned model manifest and its SHA-256 digest.

## Compatibility and storage identity

Legacy flat IDs remain valid aliases:

| Canonical reference | Legacy alias | Stored install ID |
|---|---|---|
| `whisper:tiny` | `whisper-tiny` | `whisper-tiny` |
| `qwen3-tts:0.6b-base` | `qwen3-tts-0.6b-base` | `qwen3-tts-0.6b-base` |
| `openvoice:2` | `openvoice` | `openvoice` |

Aliases resolve before installation. They therefore share the same installed record, pinned snapshot, runner and content-addressed blobs; changing the spelling never creates a second copy.

## Client commands

```bash
tako library sync
tako library models
tako library show whisper:small
tako pull whisper:small
tako plan whisper:small
tako rm whisper:small
```

`library sync` validates the complete index before atomically replacing the local cache. Pull performs a best-effort refresh and falls back to the last validated cache or the bundled index if the network is unavailable.

Set `TAKOKIT_REGISTRY_OFFLINE=1` to forbid remote refreshes. A custom endpoint can be supplied with `TAKOKIT_REGISTRY_URL` for testing or an approved mirror.

## Registry document

The version 1 index is published at `/v1/registry.json` and bundled in the repository as `registry/index.json`.

Top-level fields:

- `schema_version` — protocol version, currently `1`
- `namespace` — registry namespace
- `generated_at` — publication timestamp
- `models` — model-family records

Each family declares its name, aliases, capabilities, default tag and immutable tag records. Each tag contains its canonical target install ID, aliases, runner/adapter metadata, hardware guidance, pinned upstream source, full TOML manifest and a `sha256:` digest of that manifest.

The client rejects malformed documents, duplicate families/tags/aliases, missing defaults, invalid digests and manifests whose embedded ID disagrees with their target.

## Publishing flow

1. Add or update the pinned TOML manifest under `registry/models/`.
2. Publish a new immutable tag in `registry/index.json`.
3. Move `default_tag` only after install and real inference evidence passes.
4. Keep old tags available so existing commands and evidence remain reproducible.
5. Run workspace, registry and companion-site CI before deployment.

Do not rewrite a published tag to point at different bytes. Publish a new tag and deliberately update the family default.

## Companion site

The site under `site/` renders searchable family pages from the same registry document used by the CLI. It does not host multi-gigabyte model weights and is not a second source of truth. Its API adds CORS and cache headers while proxying the version-controlled registry document.
