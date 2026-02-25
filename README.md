# cirup-rs

A translation continuous integration tool for extracting, diffing, and merging localized resources.

## Build

```bash
cargo build --workspace
```

## Test and quality checks

```bash
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets
```

## Configuration

Most commands require a configuration file:

```toml
[vcs]
# The version control system to use
plugin = "git"
# The local path for the repository
local_path = "/opt/wayk/i18n/WaykNow"
# The remote path to the repository
remote_path = "git@bitbucket.org:devolutions/wayknow.git"

[sync]
# The source language
source_language = "en"
# The target language(s)
target_languages = [ "fr", "de" ]
# A regex to match your language files
match_language_file = "\\.json$"
# A regex to match the language from the language filename
match_language_name = "(.+?)(\\.[^.]*$|$)"
# The relative path to the language files in the repository
source_dir = "resources/i18n"
# The location to export and import translations from
working_dir = "/opt/wayk/i18n/WaykNow-Translations"

[query]
# Query backend: rusqlite | turso-local | turso-remote
backend = "rusqlite"

[query.turso]
# Required when using turso-remote
url = "libsql://your-org.turso.io"
auth_token = ""
```

### Query backend notes

- Default backend is `rusqlite`.
- Enable Turso backends (`turso-local` and `turso-remote`) with the `turso-rust` feature:

```bash
cargo run -p cirup_cli --features turso-rust -- --config ./config.cirup pull
```

- For file commands without a config, you can override the default at runtime:

```bash
set CIRUP_QUERY_BACKEND=turso-local
cirup file-diff a.json b.json
```

- For `turso-remote` without config, set the connection values via env vars:

```bash
set CIRUP_QUERY_BACKEND=turso-remote
set CIRUP_TURSO_URL=libsql://your-org.turso.io
set CIRUP_TURSO_AUTH_TOKEN=your-token
cirup file-diff a.json b.json
```

## Large RESX benchmark fixtures

Benchmark fixtures are stored in:

- `cirup_core/test/benchmark/rdm_resx`

Quick fixture sanity check:

```bash
cargo test -p cirup_core benchmark_fixture_set_is_present
```

Run large-file performance benchmark (rusqlite):

```bash
cargo test -p cirup_core benchmark_performance_rusqlite_large_resx -- --ignored --nocapture
```

Run large-file correctness benchmark (rusqlite vs turso-local):

```bash
cargo test -p cirup_core --features turso-rust benchmark_correctness_rusqlite_vs_turso_local -- --ignored --nocapture
```

## Main commands

### `vcs-log`

Shows version control history for the source language file.
You must specify an old commit and may optionally provide a new commit.

Commits are listed newest first as:
`%commit - %date - %author - %message`

Example:

```bash
cirup vcs-log --old-commit ac8d579fd --limit 20
```

### `vcs-diff`

Diffs two commits of the source language file.
You must specify an old commit and may optionally provide a new commit.

### `pull`

Generates translation files for target languages.
You can specify a commit range, and optionally include changed strings with `--show-changes`.

Example:

```bash
cirup pull --old-commit ac8d579fd --show-changes
```

### `push`

Merges translated files from the working directory back into version control.
You can specify a commit range to merge a specific set of changes.

### Other commands

Additional file-level operations are available through the cirup query engine.
See command help for full details:

```bash
cirup --help
```

## GitHub Actions

This repository includes two workflows:

- `CI` (`.github/workflows/ci.yml`)
	- Runs on push and pull request.
	- Validates formatting, clippy, and tests.

- `Release` (`.github/workflows/release.yml`)
	- Runs when pushing tags that start with `v` (for example `v0.1.0`).
	- Builds and packages `cirup` for:
		- `linux-x64`, `linux-arm64`
		- `windows-x64`, `windows-arm64`
		- `macos-x64`, `macos-arm64`
	- Publishes zip artifacts and a `checksums.txt` file to GitHub Releases.

## Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

After the tag is pushed, the release workflow builds artifacts for all supported platforms and publishes them automatically.
