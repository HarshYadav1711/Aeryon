$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
cargo clippy --workspace --all-targets -- -D warnings @args
