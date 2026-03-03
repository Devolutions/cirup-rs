# Devolutions.Cirup.Tool

`Devolutions.Cirup.Tool` is a RID-specific .NET tool package for `cirup`.

## Install

```powershell
dotnet tool install -g Devolutions.Cirup.Tool
```

## Run

```powershell
cirup --help
```

## One-shot run

```powershell
dotnet tool exec Devolutions.Cirup.Tool -- --help
```

or with the .NET 10 shortcut:

```powershell
dnx Devolutions.Cirup.Tool --help
```

## Runtime selection

The package uses RID-specific tool packaging. The .NET CLI automatically selects the best package for the current platform.

Supported RIDs:

- `win-x64`
- `win-arm64`
- `linux-x64`
- `linux-arm64`
- `osx-x64`
- `osx-arm64`

An `any` fallback package is also produced. It provides a managed fallback message on unsupported runtimes.
