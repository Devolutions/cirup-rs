# Devolutions.Cirup.Build

`Devolutions.Cirup.Build` packages the cross-platform `cirup` executable and provides `buildTransitive` MSBuild targets for sorting and resource-file operations.

## What it does

- Runs `cirup file-sort` on each file declared in `@(CirupResx)`.
- Exposes optional diff/changed/merge/subtract/convert targets for explicit build steps.
- Resolves the executable for the current host OS/architecture (build machine), not the project target RID.
- Executes sorting before build.
- Fails the build if `cirup` fails.

## Usage

Add the package and an explicit `.resx` list in your project:

```xml
<ItemGroup>
  <PackageReference Include="Devolutions.Cirup.Build" Version="1.2.3" PrivateAssets="all" />

  <CirupResx Include="Properties\Resources.resx" />
  <CirupResx Include="Properties\Resources.fr.resx" />
</ItemGroup>
```

## Exposed MSBuild targets

The package exposes the following targets:

- `CirupSortResx` (auto-runs before build when `@(CirupResx)` is defined)
- `CirupDiffResx`
- `CirupChangedValues`
- `CirupMergeResx`
- `CirupSubtractResx`
- `CirupConvertResources`
- `CirupSyncResources` (composite target that runs all Cirup targets)

Example item definitions:

```xml
<ItemGroup>
  <CirupResx Include="Properties\Resources.resx" />

  <CirupDiffResx Include="Properties\Resources.resx">
    <CompareTo>Properties\Resources.fr.resx</CompareTo>
    <Destination>artifacts\cirup\missing.fr.restext</Destination>
  </CirupDiffResx>

  <CirupChangedValues Include="Properties\Resources.resx">
    <CompareTo>Properties\Resources.fr.resx</CompareTo>
    <Destination>artifacts\cirup\changed.fr.restext</Destination>
  </CirupChangedValues>

  <CirupMergeResx Include="Properties\Resources.resx">
    <MergeFrom>Properties\Resources.fr.resx</MergeFrom>
    <Destination>artifacts\cirup\merged.resx</Destination>
  </CirupMergeResx>

  <CirupSubtractResx Include="Properties\Resources.fr.resx">
    <CompareTo>Properties\Resources.resx</CompareTo>
    <Destination>artifacts\cirup\fr-only.restext</Destination>
  </CirupSubtractResx>

  <CirupConvertResources Include="Properties\Resources.resx">
    <Destination>artifacts\cirup\Resources.restext</Destination>
  </CirupConvertResources>
</ItemGroup>
```

Run explicit targets with:

```powershell
dotnet msbuild -t:CirupDiffResx;CirupChangedValues;CirupMergeResx;CirupSubtractResx;CirupConvertResources
```

Item metadata contract:

- `CirupDiffResx`: `CompareTo` (required), `Destination` (optional)
- `CirupChangedValues`: `CompareTo` (required), `Destination` (optional)
- `CirupMergeResx`: `MergeFrom` (required), `Destination` (optional, defaults to in-place)
- `CirupSubtractResx`: `CompareTo` (required), `Destination` (optional)
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