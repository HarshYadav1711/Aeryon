$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
python -m pip install (Join-Path $root "ml")
