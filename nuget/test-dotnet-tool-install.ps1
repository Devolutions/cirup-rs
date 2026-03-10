param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$FeedDir = (Join-Path $PSScriptRoot "..\dist\nuget"),

    [string]$WorkRoot = (Join-Path $PSScriptRoot "..\target\tmp\dotnet-tool-install")
)

$ErrorActionPreference = "Stop"

$feedPath = Resolve-Path $FeedDir | Select-Object -ExpandProperty Path
$packageName = "Devolutions.Cirup.Tool.$Version.nupkg"
$packagePath = Join-Path $feedPath $packageName

if (-not (Test-Path -Path $packagePath -PathType Leaf)) {
    throw "Expected tool package not found: $packagePath"
}

if (Test-Path -Path $WorkRoot) {
    Remove-Item -Path $WorkRoot -Recurse -Force
}

$toolPath = Join-Path $WorkRoot "tools"
New-Item -Path $toolPath -ItemType Directory -Force | Out-Null

Write-Host "Using dotnet SDK $(dotnet --version)"
Write-Host "Installing $packageName from $feedPath"

dotnet tool install `
    --tool-path $toolPath `
    Devolutions.Cirup.Tool `
    --version $Version `
    --add-source $feedPath `
    --ignore-failed-sources

if ($LASTEXITCODE -ne 0) {
    throw "dotnet tool install failed with exit code $LASTEXITCODE"
}

$toolExecutable = if ($IsWindows) {
    Join-Path $toolPath "cirup.exe"
}
else {
    Join-Path $toolPath "cirup"
}

if (-not (Test-Path -Path $toolExecutable -PathType Leaf)) {
    throw "Installed tool executable not found: $toolExecutable"
}

& $toolExecutable --help | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Installed tool execution failed with exit code $LASTEXITCODE"
}

Write-Host "Dotnet tool install smoke test succeeded."