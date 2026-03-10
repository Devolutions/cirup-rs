# Devolutions.Cirup.Tool

`Devolutions.Cirup.Tool` is a .NET tool package for `cirup`.

## Install

```powershell
dotnet tool install -g Devolutions.Cirup.Tool
```

## Run

```powershell
cirup --help
```

## One-shot run (.NET 10+)

```powershell
dotnet tool exec Devolutions.Cirup.Tool -- --help
```

or with the .NET 10 shortcut:

```powershell
dnx Devolutions.Cirup.Tool --help
```

## Runtime selection

The package includes native `cirup` executables for each supported platform and selects the right one at runtime.

Supported RIDs:

- `win-x64`
- `win-arm64`
- `linux-x64`
- `linux-arm64`
- `osx-x64`
- `osx-arm64`

The managed launcher prints a guidance message on unsupported runtimes.
