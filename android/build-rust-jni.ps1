param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",

    [string]$AndroidApi = "26",

    [string[]]$Targets = @("aarch64-linux-android")
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$sdkRoot = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { Join-Path $env:LOCALAPPDATA "Android\Sdk" }
$ndkRoot = if ($env:ANDROID_NDK_HOME) {
    $env:ANDROID_NDK_HOME
} else {
    $ndkDir = Join-Path $sdkRoot "ndk"
    $latest = Get-ChildItem -LiteralPath $ndkDir -Directory | Sort-Object Name -Descending | Select-Object -First 1
    if (-not $latest) {
        throw "Android NDK not found under $ndkDir. Install NDK or set ANDROID_NDK_HOME."
    }
    $latest.FullName
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo not found on PATH. Install Rust and run rustup target add for the requested Android targets."
}

$hostTag = "windows-x86_64"
$toolchainBin = Join-Path $ndkRoot "toolchains\llvm\prebuilt\$hostTag\bin"

$abiByTarget = @{
    "aarch64-linux-android" = "arm64-v8a"
    "armv7-linux-androideabi" = "armeabi-v7a"
    "i686-linux-android" = "x86"
    "x86_64-linux-android" = "x86_64"
}

$linkerByTarget = @{
    "aarch64-linux-android" = "aarch64-linux-android$AndroidApi-clang.cmd"
    "armv7-linux-androideabi" = "armv7a-linux-androideabi$AndroidApi-clang.cmd"
    "i686-linux-android" = "i686-linux-android$AndroidApi-clang.cmd"
    "x86_64-linux-android" = "x86_64-linux-android$AndroidApi-clang.cmd"
}

foreach ($target in $Targets) {
    if (-not $abiByTarget.ContainsKey($target)) {
        throw "Unsupported Android Rust target: $target"
    }

    $linker = Join-Path $toolchainBin $linkerByTarget[$target]
    if (-not (Test-Path -LiteralPath $linker)) {
        throw "NDK linker not found: $linker"
    }

    $envName = "CARGO_TARGET_$($target.ToUpperInvariant().Replace('-', '_'))_LINKER"
    Set-Item -Path "Env:$envName" -Value $linker

    $cargoArgs = @("build", "--lib", "--target", $target)
    if ($Profile -eq "release") {
        $cargoArgs += "--release"
    }

    Push-Location $repoRoot
    try {
        & cargo @cargoArgs
    } finally {
        Pop-Location
    }

    $profileDir = if ($Profile -eq "release") { "release" } else { "debug" }
    $source = Join-Path $repoRoot "target\$target\$profileDir\libai_terminal.so"
    $destDir = Join-Path $PSScriptRoot "app\src\main\jniLibs\$($abiByTarget[$target])"
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item -LiteralPath $source -Destination (Join-Path $destDir "libai_terminal.so") -Force
    Write-Output "copied $source -> $destDir"
}
