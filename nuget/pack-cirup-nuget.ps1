param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$ArtifactsRoot = (Join-Path $PSScriptRoot "..\dist"),
    [string]$StagingRoot = (Join-Path $PSScriptRoot "staging"),
    [string]$OutputDir = (Join-Path $PSScriptRoot "..\dist\nuget")
)

$ErrorActionPreference = "Stop"

function Get-ArchivePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [Parameter(Mandatory = $true)]
        [string]$ArchiveName
    )

    $match = Get-ChildItem -Path $Root -Recurse -File -Filter $ArchiveName | Select-Object -First 1
    if (-not $match) {
        throw "Archive not found under '$Root': $ArchiveName"
    }

    return $match.FullName
}

$packageProject = Join-Path $PSScriptRoot "Devolutions.Cirup.Build.Package.csproj"
$extractRoot = Join-Path $stagingRoot "extract"
$toolsRoot = Join-Path $stagingRoot "tools\cirup"

if (Test-Path -Path $stagingRoot) {
    Remove-Item -Path $stagingRoot -Recurse -Force
}

New-Item -Path $extractRoot -ItemType Directory -Force | Out-Null
New-Item -Path $toolsRoot -ItemType Directory -Force | Out-Null
New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null

$archives = @(
    @{ Archive = "cirup-windows-x64.zip"; Rid = "win-x64"; Binary = "cirup.exe" },
    @{ Archive = "cirup-windows-arm64.zip"; Rid = "win-arm64"; Binary = "cirup.exe" },
    @{ Archive = "cirup-linux-x64.zip"; Rid = "linux-x64"; Binary = "cirup" },
    @{ Archive = "cirup-linux-arm64.zip"; Rid = "linux-arm64"; Binary = "cirup" },
    @{ Archive = "cirup-macos-x64.zip"; Rid = "osx-x64"; Binary = "cirup" },
    @{ Archive = "cirup-macos-arm64.zip"; Rid = "osx-arm64"; Binary = "cirup" }
)

foreach ($entry in $archives) {
    $archivePath = Get-ArchivePath -Root $ArtifactsRoot -ArchiveName $entry.Archive
    $extractDir = Join-Path $extractRoot $entry.Rid
    $destDir = Join-Path $toolsRoot $entry.Rid
    $destPath = Join-Path $destDir $entry.Binary

    New-Item -Path $extractDir -ItemType Directory -Force | Out-Null
    New-Item -Path $destDir -ItemType Directory -Force | Out-Null

    Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

    $sourcePath = Join-Path $extractDir $entry.Binary
    if (-not (Test-Path -Path $sourcePath)) {
        $nested = Get-ChildItem -Path $extractDir -Recurse -File -Filter $entry.Binary | Select-Object -First 1
        if (-not $nested) {
            throw "Unable to find '$($entry.Binary)' in archive '$archivePath'."
        }
        $sourcePath = $nested.FullName
    }

    Copy-Item -Path $sourcePath -Destination $destPath -Force
}

Write-Host "Packing Devolutions.Cirup.Build $Version"
dotnet pack $packageProject `
    -c Release `
    -p:Version=$Version `
    -p:CirupNugetStagingDir=$stagingRoot `
    -o $OutputDir

if ($LASTEXITCODE -ne 0) {
    throw "dotnet pack failed with exit code $LASTEXITCODE"
}

Write-Host "Created package(s) under: $OutputDir"