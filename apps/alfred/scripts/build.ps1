param([switch]$Online, [switch]$ResolveLocks, [switch]$GenerateWrapper, [switch]$Clean)
$ErrorActionPreference = 'Stop'
$appRoot = Split-Path $PSScriptRoot -Parent
$repoRoot = [IO.Path]::GetFullPath((Join-Path $appRoot '../..'))
$toolsRoot = Join-Path $repoRoot '.tools/alfred'
if (-not $env:JAVA_HOME) { $env:JAVA_HOME = Join-Path $toolsRoot 'jdk-17.0.16+8' }
if (-not $env:ANDROID_HOME) { $env:ANDROID_HOME = Join-Path $toolsRoot 'sdk' }
if (-not $env:GRADLE_USER_HOME) { $env:GRADLE_USER_HOME = Join-Path $toolsRoot 'gradle-user-home' }
if (-not $env:CARGO_HOME -and (Test-Path (Join-Path $repoRoot '.tools/cargo'))) {
    $env:CARGO_HOME = Join-Path $repoRoot '.tools/cargo'
    $env:RUSTUP_HOME = Join-Path $repoRoot '.tools/rustup'
    $env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
}
$env:RUSTUP_TOOLCHAIN = '1.85.0'
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$ndk = Join-Path $env:ANDROID_HOME 'ndk/30.0.16248370'
$clang = Join-Path $ndk 'toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android30-clang.cmd'
foreach ($file in @((Join-Path $env:JAVA_HOME 'bin/java.exe'), $clang, (Join-Path $env:ANDROID_HOME 'platforms/android-36/android.jar'), (Join-Path $env:ANDROID_HOME 'build-tools/36.0.0/zipalign.exe'))) {
    if (-not (Test-Path $file)) { throw "Missing prerequisite: $file. See BUILDING.md." }
}
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $clang
function Assert-Wrapper {
    if (-not (Test-Path (Join-Path $appRoot 'gradlew.bat'))) { throw 'Missing committed Gradle wrapper' }
    if ((Get-FileHash (Join-Path $appRoot 'gradle/wrapper/gradle-wrapper.jar')).Hash.ToLowerInvariant() -ne '81a82aaea5abcc8ff68b3dfcb58b3c3c429378efd98e7433460610fecd7ae45f') {
        throw 'Gradle wrapper JAR checksum mismatch'
    }
}
if ($Clean) {
    Assert-Wrapper
    Push-Location $appRoot
    try {
        & ./gradlew.bat --no-daemon --console=plain --offline clean
        if ($LASTEXITCODE -ne 0) { throw 'Android clean failed' }
    } finally { Pop-Location }
}
Push-Location (Join-Path $appRoot 'native')
try {
    $cargoArguments = @('build', '--release', '--locked', '--target', 'aarch64-linux-android')
    if (-not $Online) { $cargoArguments += '--offline' }
    & cargo @cargoArguments
    if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
} finally { Pop-Location }
$jniDirectory = Join-Path $appRoot 'app/build/generated/jniLibs/arm64-v8a'
New-Item -ItemType Directory -Force $jniDirectory | Out-Null
Copy-Item -LiteralPath (Join-Path $appRoot 'native/target/aarch64-linux-android/release/libalfred_native.so') -Destination $jniDirectory
$debugKey = Join-Path $toolsRoot 'debug.keystore'
if (-not (Test-Path $debugKey)) {
    New-Item -ItemType Directory -Force $toolsRoot | Out-Null
    & (Join-Path $env:JAVA_HOME 'bin/keytool.exe') -genkeypair -keystore $debugKey -storepass android -alias androiddebugkey -keypass android -dname 'CN=Android Debug,O=Android,C=US' -keyalg RSA -keysize 2048 -validity 10000
    if ($LASTEXITCODE -ne 0) { throw 'Development debug key generation failed' }
}
Push-Location $appRoot
try {
    if ($GenerateWrapper) {
        & (Join-Path $toolsRoot 'gradle-8.13/bin/gradle.bat') wrapper --gradle-version 8.13 --distribution-type bin --gradle-distribution-sha256-sum 20f1b1176237254a6fc204d8434196fa11a4cfb387567519c61556e8710aed78
        if ($LASTEXITCODE -ne 0) { throw 'Wrapper generation failed' }
    }
    Assert-Wrapper
    $gradleArguments = @('--no-daemon', '--console=plain', ':app:assembleDebug', ':app:lintDebug')
    if (-not $Online) { $gradleArguments += '--offline' }
    if ($ResolveLocks) { $gradleArguments += @('--write-locks', '--write-verification-metadata', 'sha256') }
    & ./gradlew.bat @gradleArguments
    if ($LASTEXITCODE -ne 0) { throw 'Android build/lint failed' }
    & python scripts/verify_apk.py app/build/outputs/apk/debug/app-debug.apk --sdk $env:ANDROID_HOME --smoke-assets
    if ($LASTEXITCODE -ne 0) { throw 'APK verification failed' }
    $receiptDirectory = Join-Path $appRoot 'build/receipts'
    New-Item -ItemType Directory -Force $receiptDirectory | Out-Null
    $revision = (& git -C $repoRoot rev-parse HEAD).Trim()
    $dirty = @(& git -C $repoRoot status --porcelain)
    [ordered]@{
        core_revision = $revision
        working_tree_dirty = ($dirty.Count -gt 0)
        changed_paths = $dirty
        rust = (& rustc --version)
        ndk = '30.0.16248370'
        native_api = 30
        compile_target_api = 36
        abi = 'arm64-v8a'
        panic = 'unwind'
        apk_sha256 = (Get-FileHash 'app/build/outputs/apk/debug/app-debug.apk').Hash.ToLowerInvariant()
        core_manifest_sha256 = (Get-FileHash (Join-Path $repoRoot 'Cargo.toml')).Hash.ToLowerInvariant()
        core_lock_sha256 = (Get-FileHash (Join-Path $repoRoot 'Cargo.lock')).Hash.ToLowerInvariant()
        native_lock_sha256 = (Get-FileHash 'native/Cargo.lock').Hash.ToLowerInvariant()
    } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $receiptDirectory 'build.json')
} finally { Pop-Location }
