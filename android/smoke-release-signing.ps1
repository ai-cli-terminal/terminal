# Android release signing smoke with a throwaway local keystore.
#
# This validates the Gradle signing wiring without using real release secrets.
# It writes only under artifacts/android-signing-smoke and android/app/build.
param(
  [string]$SmokeRoot = '',
  [string]$EvidencePath = '',
  [string]$AndroidHome = ''
)

$ErrorActionPreference = 'Stop'

$androidDir = $PSScriptRoot
$repoRoot = Split-Path $androidDir -Parent
if ([string]::IsNullOrWhiteSpace($SmokeRoot)) {
  $SmokeRoot = Join-Path $repoRoot 'artifacts\android-signing-smoke'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $SmokeRoot 'android-signing-smoke-evidence.json'
}
if ([string]::IsNullOrWhiteSpace($AndroidHome)) {
  $AndroidHome = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
}

$resolvedSmokeRoot = [System.IO.Path]::GetFullPath($SmokeRoot)
$resolvedRepoRoot = [System.IO.Path]::GetFullPath($repoRoot)
if (-not $resolvedSmokeRoot.StartsWith($resolvedRepoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "SmokeRoot must stay under repo root: $resolvedRepoRoot"
}

$keytool = Get-Command keytool -ErrorAction Stop
$apksigner = Join-Path $AndroidHome 'build-tools\35.0.0\apksigner.bat'
$aapt = Join-Path $AndroidHome 'build-tools\35.0.0\aapt.exe'
foreach ($path in @($apksigner, $aapt)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "required Android build tool not found: $path"
  }
}

if (Test-Path -LiteralPath $resolvedSmokeRoot) {
  Remove-Item -LiteralPath $resolvedSmokeRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $resolvedSmokeRoot | Out-Null

$keystore = Join-Path $resolvedSmokeRoot 'throwaway-release-smoke.jks'
$storePassword = 'android-signing-smoke-store-pass'
$keyAlias = 'android-signing-smoke'
$keyPassword = $storePassword

& $keytool.Source `
  -genkeypair `
  -v `
  -keystore $keystore `
  -storepass $storePassword `
  -keypass $keyPassword `
  -alias $keyAlias `
  -keyalg RSA `
  -keysize 2048 `
  -validity 2 `
  -dname 'CN=AI Terminal Android Signing Smoke, OU=Smoke, O=AI Terminal, L=Local, ST=Local, C=US' | Out-Null

$env:ANDROID_HOME = $AndroidHome
$env:ANDROID_SDK_ROOT = $AndroidHome
$env:AI_TERMINAL_ANDROID_KEYSTORE = $keystore
$env:AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD = $storePassword
$env:AI_TERMINAL_ANDROID_KEY_ALIAS = $keyAlias
$env:AI_TERMINAL_ANDROID_KEY_PASSWORD = $keyPassword

Push-Location $androidDir
try {
  & .\gradlew.bat :app:verifyFdroidReleaseInputs :app:assembleRelease :app:verifyNativeLibraries
  if ($LASTEXITCODE -ne 0) {
    throw "Gradle signing smoke failed with exit code $LASTEXITCODE"
  }
} finally {
  Pop-Location
}

$signedApk = Join-Path $androidDir 'app\build\outputs\apk\release\app-release.apk'
if (-not (Test-Path -LiteralPath $signedApk -PathType Leaf)) {
  throw "signed release APK was not generated: $signedApk"
}

$verifyOutput = & $apksigner verify --verbose $signedApk 2>&1
if ($LASTEXITCODE -ne 0) {
  $verifyOutput | Set-Content -LiteralPath (Join-Path $resolvedSmokeRoot 'apksigner-verify.txt') -Encoding utf8
  throw "apksigner verify failed; see android-signing-smoke evidence"
}

$badging = & $aapt dump badging $signedApk
$packageLine = ($badging | Select-String -Pattern '^package:' | Select-Object -First 1).Line
$apkItem = Get-Item -LiteralPath $signedApk
$apkHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $signedApk).Hash.ToLowerInvariant()

$evidence = [pscustomobject]@{
  status = 'passed'
  timestamp = (Get-Date).ToString('o')
  signedApk = $signedApk
  apkSize = $apkItem.Length
  apkSha256 = $apkHash
  package = $packageLine
  apksigner = $verifyOutput
  keystore = $keystore
  keyAlias = $keyAlias
  note = 'Throwaway smoke keystore only; do not use for real releases.'
}
$evidence | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
Write-Output "ANDROID_SIGNING_SMOKE_OK $EvidencePath"
