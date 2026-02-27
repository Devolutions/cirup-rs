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

## Query backend configuration

- Default backend is `turso-local`.
- Default build enables `turso-rust`, so no C-backed SQLite dependency is built by default.
- Enable `rusqlite` (C-backed SQLite) explicitly with the `rusqlite-c` feature:

```bash
cargo run -p cirup_cli --features rusqlite-c -- file-diff a.json b.json
```

- Turso backends (`turso-local` and `turso-remote`) are available with default features:

```bash
cargo run -p cirup_cli -- file-diff a.json b.json
```

- You can override the backend at runtime:

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

The release pipeline creates a `Devolutions.Cirup.Build` NuGet package that contains all supported platform binaries and MSBuild `buildTransitive` targets.
The target always runs the executable for the current host machine (build environment), not `$(RuntimeIdentifier)`.

Add it to a project and declare explicit resource files:

```xml
<ItemGroup>
	<PackageReference Include="Devolutions.Cirup.Build" Version="1.2.3" PrivateAssets="all" />

	<CirupResources Include="Properties\Resources.resx" />
	<CirupResources Include="Properties\Resources.fr.resx" />
</ItemGroup>
```

This runs `cirup file-sort` on each `@(CirupResources)` file before build and fails the build on errors.
The package also exposes explicit targets for diff, changed values, merge, subtract, convert, and a composite sync target.

### End-to-end NuGet validation

Run the local end-to-end script to validate package packing and MSBuild execution against a sample .NET project:

```bash
pwsh ./nuget/test-e2e.ps1
```

The script packs `Devolutions.Cirup.Build` to a local feed under `target/tmp/nuget-e2e`, restores/builds `nuget/samples/Devolutions.Cirup.Build.E2E`, runs all Cirup targets, verifies generated artifacts, and ensures no `cirup` executable is copied into build outputs.

## Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

After the tag is pushed, the release workflow builds artifacts for all supported platforms and publishes them automatically.
