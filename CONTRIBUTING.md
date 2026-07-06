# Contributing

Takokit should stay modular and Rust-first.

## Development

```bash
cargo check
cargo test
cargo run -p takokit-cli -- status
```

Frontend:

```bash
cd apps/desktop
npm install
npm run build
```

## Guidelines

- Keep CLI, server, desktop UI, and runner concerns separated.
- Add model integrations behind adapter traits.
- Do not put model-specific logic in API handlers or UI pages.
- Use typed errors for intentional gaps.
- Keep docs updated when adding a new runner or model family.

