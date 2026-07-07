$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

& (Join-Path $PSScriptRoot "cargo-fmt.ps1") -- --check
& (Join-Path $PSScriptRoot "cargo-test.ps1")
& (Join-Path $PSScriptRoot "cargo-clippy.ps1")
& (Join-Path $PSScriptRoot "cmake-build.ps1")
& (Join-Path $PSScriptRoot "python-install.ps1")
& (Join-Path $PSScriptRoot "npm-install.ps1")

Set-Location (Join-Path $root "frontend")
npm run build
