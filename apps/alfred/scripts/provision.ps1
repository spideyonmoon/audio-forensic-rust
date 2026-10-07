param([switch]$AcceptAndroidLicenses)
$ErrorActionPreference = 'Stop'
if (-not $AcceptAndroidLicenses) {
    throw 'Review the Android SDK license before running with -AcceptAndroidLicenses: https://developer.android.com/studio/terms'
}
$appRoot = Split-Path $PSScriptRoot -Parent
$repoRoot = [IO.Path]::GetFullPath((Join-Path $appRoot '../..'))
$toolsRoot = Join-Path $repoRoot '.tools/alfred'
$downloads = Join-Path $toolsRoot 'downloads'
$spec = Get-Content (Join-Path $PSScriptRoot 'toolchain-windows.json') -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Force $downloads | Out-Null
function Get-Archive($url, $hash, $algorithm, $name, $expectedBytes) {
    $archive = Join-Path $downloads $name
    if (-not (Test-Path $archive) -or (Get-Item $archive).Length -lt $expectedBytes) {
        & curl.exe --continue-at - --fail --location --connect-timeout 30 --max-time 600 --retry 2 --output $archive $url
        if ($LASTEXITCODE -ne 0) { throw "Download failed: $name" }
    }
    if ((Get-FileHash $archive -Algorithm $algorithm).Hash.ToLowerInvariant() -ne $hash) {
        throw "Checksum mismatch: $name. Remove the incomplete archive explicitly and retry."
    }
    return $archive
}
$jdk = Get-Archive $spec.jdk.url $spec.jdk.sha256 'SHA256' 'jdk.zip' $spec.jdk.bytes
if (-not (Test-Path (Join-Path $toolsRoot 'jdk-17.0.16+8/bin/java.exe'))) {
    & tar.exe -xf $jdk -C $toolsRoot
    if ($LASTEXITCODE -ne 0) { throw "JDK extraction failed" }
}
$gradle = Get-Archive $spec.gradle.url $spec.gradle.sha256 'SHA256' 'gradle.zip' $spec.gradle.bytes
if (-not (Test-Path (Join-Path $toolsRoot 'gradle-8.13/bin/gradle.bat'))) {
    & tar.exe -xf $gradle -C $toolsRoot
    if ($LASTEXITCODE -ne 0) { throw "Gradle extraction failed" }
}
$sdk = Join-Path $toolsRoot 'sdk'
foreach ($entry in $spec.android) {
    $name = $entry.url.Split('/')[-1]
    $archive = Get-Archive $entry.url $entry.sha1 'SHA1' $name $entry.bytes
    if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) {
        throw "SDK SHA256 mismatch: $name"
    }
    # Exact repository archive revisions/checksums, not sdkmanager's moving catalog.
    switch ($entry.package) {
        'cmdline-tools;19.0' { $relative = 'cmdline-tools/19.0'; $prefix = 'cmdline-tools' }
        'platforms;android-36' { $relative = 'platforms/android-36'; $prefix = 'android-36' }
        'build-tools;36.0.0' { $relative = 'build-tools/36.0.0'; $prefix = 'android-16' }
        'ndk;30.0.16248370' { $relative = 'ndk/30.0.16248370'; $prefix = 'android-ndk-r30' }
        default { throw 'Unknown pinned package' }
    }
    $destination = Join-Path $sdk $relative
    if (-not (Test-Path (Join-Path $destination 'source.properties'))) {
        $unpack = Join-Path $toolsRoot ('unpack-' + $entry.package.Replace(';', '-'))
        New-Item -ItemType Directory -Force $unpack | Out-Null
        & tar.exe -xf $archive -C $unpack
        if ($LASTEXITCODE -ne 0) { throw "SDK extraction failed: $name" }
        $source = Join-Path $unpack $prefix
        if (-not (Test-Path (Join-Path $source 'source.properties'))) {
            throw "Unexpected archive layout for $name; inspect $unpack"
        }
        New-Item -ItemType Directory -Force (Split-Path $destination -Parent) | Out-Null
        foreach ($candidate in @($source, $destination)) {
            $absolute = [IO.Path]::GetFullPath($candidate)
            if (-not $absolute.StartsWith($toolsRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
                throw 'Move target escaped the repository tool directory'
            }
        }
        Move-Item -LiteralPath $source -Destination $destination
    }
}
$env:JAVA_HOME = Join-Path $toolsRoot 'jdk-17.0.16+8'
$env:ANDROID_HOME = $sdk
$env:GRADLE_USER_HOME = Join-Path $toolsRoot 'gradle-user-home'
# This affirmative input is only permitted by the explicit caller switch above.
1..100 | ForEach-Object { 'y' } | & (Join-Path $sdk 'cmdline-tools/19.0/bin/sdkmanager.bat') "--sdk_root=$sdk" --licenses
if ($LASTEXITCODE -ne 0) { throw 'SDK license acceptance failed' }
Get-ChildItem $downloads -File | Get-FileHash -Algorithm SHA256 | Format-Table -AutoSize
Write-Output "Provisioned local tools at $toolsRoot. No system installation or global settings changed."
