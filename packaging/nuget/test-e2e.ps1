param(
    [string]$Version = "0.0.0-local",
    [string]$Configuration = "Release"
)

$ErrorActionPreference = "Stop"

function Reset-SampleResx {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SampleDir
    )

    $neutral = @"
<?xml version="1.0" encoding="utf-8"?>
<root>
  <data name="zeta" xml:space="preserve">
    <value>Zeta</value>
  </data>
  <data name="alpha" xml:space="preserve">
    <value>Alpha</value>
  </data>
  <data name="beta" xml:space="preserve">
    <value>Beta</value>
  </data>
</root>
"@

    $fr = @"
<?xml version="1.0" encoding="utf-8"?>
<root>
  <data name="zeta" xml:space="preserve">
    <value>Zeta FR</value>
  </data>
  <data name="alpha" xml:space="preserve">
    <value>Alpha FR</value>
  </data>
  <data name="beta" xml:space="preserve">
    <value>Beta FR</value>
  </data>
</root>
"@

    Set-Content -Path (Join-Path $SampleDir "Resources\Strings.resx") -Value $neutral -NoNewline -Encoding utf8
    Set-Content -Path (Join-Path $SampleDir "Resources\Strings.fr.resx") -Value $fr -NoNewline -Encoding utf8
}

function Assert-Sorted {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $content = Get-Content -Path $Path -Raw
    $alpha = $content.IndexOf('name="alpha"')
    $beta = $content.IndexOf('name="beta"')
    $zeta = $content.IndexOf('name="zeta"')

    if ($alpha -lt 0 -or $beta -lt 0 -or $zeta -lt 0) {
        throw "Expected keys alpha/beta/zeta were not all found in $Path"
    }

    if (-not ($alpha -lt $beta -and $beta -lt $zeta)) {
        throw "RESX file is not sorted by key: $Path"
    }
}

$scriptRoot = $PSScriptRoot
if (-not $scriptRoot) {
    $scriptRoot = Split-Path -Path $MyInvocation.MyCommand.Path -Parent
}

$packageRoot = Resolve-Path $scriptRoot | Select-Object -ExpandProperty Path
$repoRoot = Resolve-Path (Join-Path $packageRoot "..\..") | Select-Object -ExpandProperty Path
$sampleDir = Join-Path $packageRoot "samples\Devolutions.Cirup.Build.E2E"
$sampleProject = Join-Path $sampleDir "Devolutions.Cirup.Build.E2E.csproj"
$packageProject = Join-Path $packageRoot "Devolutions.Cirup.Build.Package.csproj"

$workRoot = Join-Path $repoRoot "target\tmp\nuget-e2e"
$stagingRoot = Join-Path $workRoot "staging"
$feedDir = Join-Path $workRoot "feed"

if (Test-Path -Path $workRoot) {
    Remove-Item -Path $workRoot -Recurse -Force
}

New-Item -Path $stagingRoot -ItemType Directory -Force | Out-Null
New-Item -Path $feedDir -ItemType Directory -Force | Out-Null

$sourceBinary = Join-Path $repoRoot "target\debug\cirup.exe"
if (-not (Test-Path -Path $sourceBinary)) {
    Push-Location $repoRoot
    try {
        cargo build --package cirup_cli --bin cirup
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -Path $sourceBinary)) {
    throw "Expected CLI binary was not produced: $sourceBinary"
}

$runtimeCopies = @(
    @{ Rid = "win-x64"; Binary = "cirup.exe" },
    @{ Rid = "win-arm64"; Binary = "cirup.exe" },
    @{ Rid = "linux-x64"; Binary = "cirup" },
    @{ Rid = "linux-arm64"; Binary = "cirup" },
    @{ Rid = "osx-x64"; Binary = "cirup" },
    @{ Rid = "osx-arm64"; Binary = "cirup" }
)

foreach ($runtime in $runtimeCopies) {
    $destDir = Join-Path $stagingRoot "runtimes\$($runtime.Rid)\native"
    New-Item -Path $destDir -ItemType Directory -Force | Out-Null
    Copy-Item -Path $sourceBinary -Destination (Join-Path $destDir $runtime.Binary) -Force
}

dotnet pack $packageProject `
    -c Release `
    -p:Version=$Version `
    -p:CirupNugetStagingDir=$stagingRoot `
    -o $feedDir
if ($LASTEXITCODE -ne 0) {
    throw "dotnet pack failed with exit code $LASTEXITCODE"
}

$packagePath = Get-ChildItem -Path $feedDir -File -Filter "Devolutions.Cirup.Build.$Version.nupkg" | Select-Object -First 1
if (-not $packagePath) {
    throw "Expected package not found in local feed: Devolutions.Cirup.Build.$Version.nupkg"
}

Reset-SampleResx -SampleDir $sampleDir

Push-Location $sampleDir
try {
    dotnet restore $sampleProject -p:CirupBuildVersion=$Version -p:RestoreAdditionalProjectSources=$feedDir
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet restore failed with exit code $LASTEXITCODE"
    }

    dotnet build $sampleProject -c $Configuration --no-restore -p:CirupBuildVersion=$Version
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet build failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

Assert-Sorted -Path (Join-Path $sampleDir "Resources\Strings.resx")
Assert-Sorted -Path (Join-Path $sampleDir "Resources\Strings.fr.resx")

Write-Host "E2E validation succeeded. Package: $($packagePath.FullName)"