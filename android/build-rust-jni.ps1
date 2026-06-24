param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",

    [string]$AndroidApi = "26",

    [string[]]$Targets = @(
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "i686-linux-android",
        "x86_64-linux-android"
    ),

    [switch]$NoRustupTargetInstall
)

$ErrorActionPreference = "Stop"

function Join-Parts {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Parts)
    [System.IO.Path]::Combine($Parts)
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$sdkRoot = if ($env:ANDROID_HOME) {
    $env:ANDROID_HOME
} elseif ($env:ANDROID_SDK_ROOT) {
    $env:ANDROID_SDK_ROOT
} elseif ($env:LOCALAPPDATA) {
    Join-Parts $env:LOCALAPPDATA "Android" "Sdk"
} else {
    Join-Parts $HOME "Android" "Sdk"
}
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

$hostTag = if ($IsWindows -or $env:OS -eq "Windows_NT") {
    "windows-x86_64"
} elseif ($IsMacOS) {
    if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq "Arm64") {
        "darwin-aarch64"
    } else {
        "darwin-x86_64"
    }
} elseif ($IsLinux) {
    "linux-x86_64"
} else {
    throw "Unsupported host OS for Android NDK toolchain detection."
}
$toolchainBin = Join-Parts $ndkRoot "toolchains" "llvm" "prebuilt" $hostTag "bin"

$abiByTarget = @{
    "aarch64-linux-android" = "arm64-v8a"
    "armv7-linux-androideabi" = "armeabi-v7a"
    "i686-linux-android" = "x86"
    "x86_64-linux-android" = "x86_64"
}

$linkerByTarget = @{
    "aarch64-linux-android" = "aarch64-linux-android$AndroidApi-clang"
    "armv7-linux-androideabi" = "armv7a-linux-androideabi$AndroidApi-clang"
    "i686-linux-android" = "i686-linux-android$AndroidApi-clang"
    "x86_64-linux-android" = "x86_64-linux-android$AndroidApi-clang"
}

foreach ($target in $Targets) {
    if (-not $abiByTarget.ContainsKey($target)) {
        throw "Unsupported Android Rust target: $target"
    }

    $linkerName = $linkerByTarget[$target]
    if ($hostTag.StartsWith("windows-")) {
        $linkerName = "$linkerName.cmd"
    }
    $linker = Join-Path $toolchainBin $linkerName
    if (-not (Test-Path -LiteralPath $linker)) {
        throw "NDK linker not found: $linker"
    }

    if (-not $NoRustupTargetInstall -and (Get-Command rustup -ErrorAction SilentlyContinue)) {
        & rustup target add $target
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
    $source = Join-Parts $repoRoot "target" $target $profileDir "libai_terminal.so"
    $destDir = Join-Parts $PSScriptRoot "app" "src" "main" "jniLibs" $($abiByTarget[$target])
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item -LiteralPath $source -Destination (Join-Path $destDir "libai_terminal.so") -Force
    Write-Output "copied $source -> $destDir"
}
