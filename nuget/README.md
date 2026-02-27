# Devolutions.Cirup.Build

`Devolutions.Cirup.Build` packages the cross-platform `cirup` executable and provides `buildTransitive` MSBuild targets for sorting and resource-file operations.

## What it does

- Runs `cirup file-sort` on each file declared in `@(CirupResources)`.
- Exposes optional diff/changed/merge/subtract/convert targets for explicit build steps.
- Resolves the executable for the current host OS/architecture (build machine), not the project target RID.
- Executes sorting before build.
- Fails the build if `cirup` fails.

## Usage

Add the package and an explicit resource-file list in your project:

```xml
<ItemGroup>
  <PackageReference Include="Devolutions.Cirup.Build" Version="1.2.3" PrivateAssets="all" />

  <CirupResources Include="Properties\Resources.resx" />
  <CirupResources Include="Properties\Resources.fr.resx" />
</ItemGroup>
```

## Exposed MSBuild targets

The package exposes the following targets:

- `CirupSortResources` (auto-runs before build when `@(CirupResources)` is defined)
- `CirupDiffResources`
- `CirupChangedValues`
- `CirupMergeResources`
- `CirupSubtractResources`
- `CirupConvertResources`
- `CirupSyncResources` (composite target that runs all Cirup targets)

Example item definitions:

```xml
<ItemGroup>
  <CirupResources Include="Properties\Resources.resx" />

  <CirupDiffResources Include="Properties\Resources.resx">
    <CompareTo>Properties\Resources.fr.resx</CompareTo>
    <Destination>artifacts\cirup\missing.fr.restext</Destination>
  </CirupDiffResources>

  <CirupChangedValues Include="Properties\Resources.resx">
    <CompareTo>Properties\Resources.fr.resx</CompareTo>
    <Destination>artifacts\cirup\changed.fr.restext</Destination>
  </CirupChangedValues>

  <CirupMergeResources Include="Properties\Resources.resx">
    <MergeFrom>Properties\Resources.fr.resx</MergeFrom>
    <Destination>artifacts\cirup\merged.resx</Destination>
  </CirupMergeResources>

  <CirupSubtractResources Include="Properties\Resources.fr.resx">
    <CompareTo>Properties\Resources.resx</CompareTo>
    <Destination>artifacts\cirup\fr-only.restext</Destination>
  </CirupSubtractResources>

  <CirupConvertResources Include="Properties\Resources.resx">
    <Destination>artifacts\cirup\Resources.restext</Destination>
  </CirupConvertResources>
</ItemGroup>
```

Run explicit targets with:

```powershell
dotnet msbuild -t:CirupDiffResources;CirupChangedValues;CirupMergeResources;CirupSubtractResources;CirupConvertResources
```

Item metadata contract:

- `CirupDiffResources`: `CompareTo` (required), `Destination` (optional)
- `CirupChangedValues`: `CompareTo` (required), `Destination` (optional)
- `CirupMergeResources`: `MergeFrom` (required), `Destination` (optional, defaults to in-place)
- `CirupSubtractResources`: `CompareTo` (required), `Destination` (optional)
- `CirupConvertResources`: `Destination` (required)

## Optional MSBuild properties

```xml
<PropertyGroup>
  <CirupEnabled>true</CirupEnabled>
  <CirupHostRuntimeIdentifier></CirupHostRuntimeIdentifier>
  <CirupWorkingDirectory>$(MSBuildProjectDirectory)</CirupWorkingDirectory>
  <CirupAdditionalArgs></CirupAdditionalArgs>
  <CirupLogImportance>high</CirupLogImportance>
</PropertyGroup>
```

Set `<CirupEnabled>false</CirupEnabled>` to disable sorting.
`<CirupHostRuntimeIdentifier>` is optional and exists for advanced overrides.

## End-to-end sample

A runnable sample project is available at `nuget/samples/Devolutions.Cirup.Build.E2E`.

Run the full local validation flow from the repository root:

```powershell
./nuget/test-e2e.ps1
```