$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$buildDir = if ($args.Count -gt 0) { $args[0] } else { Join-Path $root "native\cpp-dsp\build" }

cmake -S (Join-Path $root "native\cpp-dsp") -B $buildDir
cmake --build $buildDir

$cache = Join-Path $buildDir "CMakeCache.txt"
if ((Test-Path $cache) -and (Select-String -Path $cache -Pattern "CMAKE_CONFIGURATION_TYPES" -Quiet)) {
  ctest --test-dir $buildDir -C Debug --output-on-failure
} else {
  ctest --test-dir $buildDir --output-on-failure
}
