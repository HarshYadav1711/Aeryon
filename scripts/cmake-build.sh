#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build_dir="${1:-$root/native/cpp-dsp/build}"

cmake -S "$root/native/cpp-dsp" -B "$build_dir"
cmake --build "$build_dir"

if [ -f "$build_dir/CMakeCache.txt" ] && grep -q "CMAKE_CONFIGURATION_TYPES" "$build_dir/CMakeCache.txt"; then
  ctest --test-dir "$build_dir" -C Debug --output-on-failure
else
  ctest --test-dir "$build_dir" --output-on-failure
fi
