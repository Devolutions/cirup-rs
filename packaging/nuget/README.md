# Devolutions.Cirup.Build

`Devolutions.Cirup.Build` packages the cross-platform `cirup` executable and provides a `buildTransitive` MSBuild target that sorts `.resx` files before build.

## What it does

- Runs `cirup file-sort` on each file declared in `@(CirupResx)`.
- Executes before build.
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

## Optional MSBuild properties

```xml
<PropertyGroup>
  <CirupEnabled>true</CirupEnabled>
  <CirupWorkingDirectory>$(MSBuildProjectDirectory)</CirupWorkingDirectory>
  <CirupAdditionalArgs></CirupAdditionalArgs>
  <CirupLogImportance>high</CirupLogImportance>
</PropertyGroup>
```

Set `<CirupEnabled>false</CirupEnabled>` to disable sorting.

## End-to-end sample

A runnable sample project is available at `packaging/nuget/samples/Devolutions.Cirup.Build.E2E`.

Run the full local validation flow from the repository root:

```powershell
./packaging/nuget/test-e2e.ps1
```