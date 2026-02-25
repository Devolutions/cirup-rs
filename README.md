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
