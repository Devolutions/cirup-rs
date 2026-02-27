# AGENTS.md

This file provides guidance for human and AI contributors working in this repository.

## Project overview

- Workspace: Rust Cargo workspace
- Crates:
  - `cirup_core`: core engine (query backend config, parsing formats, query logic)
  - `cirup_cli`: command-line frontend (`cirup` binary)

## Repository structure

- `cirup_cli/src/main.rs`: CLI argument parsing and command dispatch
- `cirup_core/src/config.rs`: configuration models and parsing
- `cirup_core/src/file.rs`: resource file loading/saving abstraction
- `cirup_core/src/{json,restext,resx}.rs`: format implementations
- `cirup_core/src/query.rs`: query engine and helper operations

## Development workflow

Use these commands locally before committing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

If formatting fails:

```bash
cargo fmt --all
```

## Coding guidelines

- Keep changes minimal and scoped to the task.
- Preserve existing behavior unless explicitly changing it.
- Prefer fixing root causes over suppressing lints.
- Avoid broad refactors unrelated to the current request.
- Keep code cross-platform (Linux/macOS/Windows).

## Testing guidance

- For logic changes in `cirup_core`, run `cargo test --workspace`.
- Add or update unit tests when changing parsing/query logic.
- Do not introduce flaky tests or tests that require network access.

## Lint and formatting notes

- Workspace lint settings are defined in the root `Cargo.toml` and `clippy.toml`.
- Formatting settings are in `rustfmt.toml`.
- The CI workflow enforces formatting, clippy, and tests.

## Release automation

- CI workflow: `.github/workflows/ci.yml`
- Release workflow: `.github/workflows/release.yml`
- Release trigger: push a `v*` tag (for example `v0.1.0`)
- Release workflow builds and publishes artifacts for:
  - Linux x64/arm64
  - Windows x64/arm64
  - macOS x64/arm64

## Commit style

Use short, imperative commit messages that describe intent, for example:

- `Fix resource language file filtering`
- `Add release workflow for multi-platform artifacts`
- `Reduce clippy warnings in query module`
