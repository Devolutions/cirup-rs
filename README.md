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
backend = "turso-local"

[query.turso]
# Required when using turso-remote
url = "libsql://your-org.turso.io"
auth_token = ""
```

### Query backend notes

- Default backend is `turso-local`.
- Default build enables `turso-rust`, so no C-backed SQLite dependency is built by default.
- Enable `rusqlite` (C-backed SQLite) explicitly with the `rusqlite-c` feature:

```bash
cargo run -p cirup_cli --features rusqlite-c -- --config ./config.cirup pull
```

- Turso backends (`turso-local` and `turso-remote`) are available with default features:

```bash
cargo run -p cirup_cli -- --config ./config.cirup pull
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
cargo test -p cirup_core --features rusqlite-c benchmark_performance_rusqlite_large_resx -- --ignored --nocapture
```

Run large-file correctness benchmark (rusqlite vs turso-local):

```bash
cargo test -p cirup_core --features rusqlite-c benchmark_correctness_rusqlite_vs_turso_local -- --ignored --nocapture
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
	- Performs a dry-run split packaging flow:
		- Builds platform `cirup` release archives.
		- Packs `Devolutions.Cirup.Build` from downloaded prebuilt archives (no Rust rebuild in NuGet pack step).

- `Release` (`.github/workflows/release.yml`)
	- Runs when pushing tags that start with `v` (for example `v0.1.0`).
	- Builds and packages `cirup` for:
		- `linux-x64`, `linux-arm64`
		- `windows-x64`, `windows-arm64`
		- `macos-x64`, `macos-arm64`
	- Publishes zip artifacts, a cross-platform `Devolutions.Cirup.Build.<version>.nupkg`, and a `checksums.txt` file to GitHub Releases.

## NuGet package for .NET projects

The release pipeline creates a `Devolutions.Cirup.Build` NuGet package that contains all supported platform binaries and an MSBuild `buildTransitive` target.

Add it to a project and declare explicit RESX files:

```xml
<ItemGroup>
	<PackageReference Include="Devolutions.Cirup.Build" Version="1.2.3" PrivateAssets="all" />

  <CirupResx Include="Properties\Resources.resx" />
  <CirupResx Include="Properties\Resources.fr.resx" />
</ItemGroup>
```

This runs `cirup file-sort` on each `@(CirupResx)` file before build and fails the build on errors.

### End-to-end NuGet validation

Run the local end-to-end script to validate package packing and MSBuild execution against a sample .NET project:

```bash
pwsh ./packaging/nuget/test-e2e.ps1
```

The script packs `Devolutions.Cirup.Build` to a local feed under `target/tmp/nuget-e2e`, restores/builds `packaging/nuget/samples/Devolutions.Cirup.Build.E2E`, and verifies sample `.resx` files were sorted.

## Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

After the tag is pushed, the release workflow builds artifacts for all supported platforms and publishes them automatically.
