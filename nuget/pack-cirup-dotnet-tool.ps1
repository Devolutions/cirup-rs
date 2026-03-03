param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$ArtifactsRoot = (Join-Path $PSScriptRoot "..\dist"),
    [string]$StagingRoot = (Join-Path $PSScriptRoot "staging"),
    [string]$OutputDir = (Join-Path $PSScriptRoot "..\dist\nuget")
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "scripts/Import-CirupArtifacts.ps1")

$packageProject = Join-Path $PSScriptRoot "tool/Devolutions.Cirup.Tool.csproj"

New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null

Import-CirupArtifacts -ArtifactsRoot $ArtifactsRoot -StagingRoot $StagingRoot

Write-Host "Packing Devolutions.Cirup.Tool $Version"
dotnet pack $packageProject `
    -c Release `
    -p:Version=$Version `
    -p:CirupNugetStagingDir=$StagingRoot `
    -o $OutputDir

if ($LASTEXITCODE -ne 0) {
    throw "dotnet pack failed with exit code $LASTEXITCODE"
}

Write-Host "Created package(s) under: $OutputDir"
