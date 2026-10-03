# Use the project-local toolchain when present, otherwise the caller's cargo.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$localCargo = Join-Path $root '.tools/cargo/bin/cargo.exe'
if (Test-Path -LiteralPath $localCargo) {
    $env:RUSTUP_HOME = Join-Path $root '.tools/rustup'
    $env:CARGO_HOME = Join-Path $root '.tools/cargo'
    $env:PATH = (Join-Path $root '.tools/cargo/bin') + [IO.Path]::PathSeparator + $env:PATH
    # Prefer the installed GCC driver over Rust's bundled MinGW linker on this host.
    if (!$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER) {
        $gcc = Get-Command gcc -ErrorAction SilentlyContinue
        if ($gcc) { $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $gcc.Source }
    }
    & $localCargo @args
} else {
    & cargo @args
}
exit $LASTEXITCODE
