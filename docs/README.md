# Takokit documentation

This directory contains contributor, architecture, testing, release, storage, runner, model-support, and API reference material for Takokit. Public task-oriented user documentation is rendered at `https://takokit.dawnlightlabs.com/docs` from the repository's `site/src/docs` sources.

## Start here

- [Architecture](architecture.md)
- [State and storage](state-and-storage.md)
- [Contributor guide](contributor-guide.md)
- [Testing](TESTING.md)
- [Release process](release-process.md)
- [Platform support](platform-support.md)
- [Model support](model-support.md)
- [Runner architecture](runners.md)
- [Registry](registry.md)
- [Local API](api.md)
- [API route inventory](api-route-inventory.md)
- [Custom models](custom-models.md)
- [Model smoke tests](MODEL_SMOKE_TESTS.md)
- [RVC quality gate](RVC_QUALITY_GATE.md)
- [Licenses / third-party notices](licenses.md)

## Documentation ownership

Use one source of truth whenever the fact is machine-readable:

- model names/tags/hardware/license metadata → canonical registry,
- CLI syntax → Clap command tree,
- local API schemas/routes → executable router and `/openapi.json`,
- release version/platform/artifact metadata → release manifests/index,
- platform support → `platform-support.md` plus exact package acceptance evidence.

Handwritten docs explain workflows, invariants, safety boundaries, and contributor procedures. They should not silently fork machine-readable facts.

## Release maturity

Every Takokit `0.x.x` release is **Beta**. `1.0.0` is the first Stable release target.

Beta does not mean mock or unverified packaging: public beta releases use real artifacts, signed Takokit release metadata, checksums, and platform acceptance. It does mean compatibility and UX can still evolve before the 1.0 commitment.
