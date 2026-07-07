#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$root/scripts/cargo-fmt.sh" -- --check
"$root/scripts/cargo-test.sh"
"$root/scripts/cargo-clippy.sh"
"$root/scripts/cmake-build.sh"
"$root/scripts/python-install.sh"
"$root/scripts/npm-install.sh"

cd "$root/frontend"
npm run build
