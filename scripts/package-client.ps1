[CmdletBinding()]
param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",

    [string]$OutputDir = "dist/client-windows"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$outputPath = Join-Path $repoRoot $OutputDir
$assetsSource = Join-Path $repoRoot "assets"

if (-not (Test-Path $assetsSource -PathType Container)) {
    throw "Missing assets directory at '$assetsSource'. Run this script from a full repository checkout."
}

Push-Location $repoRoot
try {
    if ($Profile -eq "release") {
        cargo build --release --bin game
    } else {
        cargo build --bin game
    }
} finally {
    Pop-Location
}

$profileTarget = if ($Profile -eq "release") { "release" } else { "debug" }
$gameExe = Join-Path $repoRoot "target/$profileTarget/game.exe"

if (-not (Test-Path $gameExe -PathType Leaf)) {
    throw "Cargo completed without producing '$gameExe'."
}

if (Test-Path $outputPath) {
    Remove-Item $outputPath -Recurse -Force
}

New-Item $outputPath -ItemType Directory -Force | Out-Null
Copy-Item $gameExe (Join-Path $outputPath "game.exe")
Copy-Item $assetsSource (Join-Path $outputPath "assets") -Recurse

$launcherPath = Join-Path $outputPath "run-client.bat"
@'
@echo off
setlocal
"%~dp0game.exe" client %*
'@ | Set-Content -Path $launcherPath -Encoding ASCII

Write-Host "Client bundle created at: $outputPath"
Write-Host "Run it with: $launcherPath --uri ws://193.70.42.29:3000 --module bevymmo-v2"
