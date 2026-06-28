# GitHub Android signing secrets preflight.
#
# Default mode reads the same secret-shaped environment variables used by
# .github/workflows/release.yml:
# - AI_TERMINAL_ANDROID_KEYSTORE_BASE64
# - AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD
# - AI_TERMINAL_ANDROID_KEY_ALIAS
# - AI_TERMINAL_ANDROID_KEY_PASSWORD
#
# Use -UseThrowawayKeystore to exercise the exact base64 decode path without
# real release secrets. Evidence never writes passwords or base64 secret values.
param(
  [switch]$UseThrowawayKeystore,
  [string]$SmokeRoot = '',
  [string]$EvidencePath = '',
  [string]$AndroidHome = ''
)

$ErrorActionPreference = 'Stop'

$androidDir = $PSScriptRoot
$repoRoot = Split-Path $androidDir -Parent
if ([string]::IsNullOrWhiteSpace($SmokeRoot)) {
  $SmokeRoot = Join-Path $repoRoot 'artifacts\android-github-signing-preflight'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $SmokeRoot 'android-github-signing-preflight-evidence.json'
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

$secretKeystoreBase64 = $env:AI_TERMINAL_ANDROID_KEYSTORE_BASE64
$secretKeystorePassword = $env:AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD
$secretKeyAlias = $env:AI_TERMINAL_ANDROID_KEY_ALIAS
$secretKeyPassword = $env:AI_TERMINAL_ANDROID_KEY_PASSWORD
$mode = 'provided-secrets'

if ($UseThrowawayKeystore) {
  $mode = 'throwaway'
  $throwawayKeystore = Join-Path $resolvedSmokeRoot 'throwaway-github-secret-source.jks'
  $secretKeystorePassword = 'android-github-secret-smoke-store-pass'
  $secretKeyAlias = 'android-github-secret-smoke'
  $secretKeyPassword = $secretKeystorePassword

  & $keytool.Source `
    -genkeypair `
    -v `
    -keystore $throwawayKeystore `
    -storepass $secretKeystorePassword `
    -keypass $secretKeyPassword `
    -alias $secretKeyAlias `
    -keyalg RSA `
    -keysize 2048 `
    -validity 2 `
    -dname 'CN=AI Terminal GitHub Secret Smoke, OU=Smoke, O=AI Terminal, L=Local, ST=Local, C=US' *> $null

  $secretKeystoreBase64 = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($throwawayKeystore))
}

$missing = @()
if ([string]::IsNullOrWhiteSpace($secretKeystoreBase64)) { $missing += 'AI_TERMINAL_ANDROID_KEYSTORE_BASE64' }
if ([string]::IsNullOrWhiteSpace($secretKeystorePassword)) { $missing += 'AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD' }
if ([string]::IsNullOrWhiteSpace($secretKeyAlias)) { $missing += 'AI_TERMINAL_ANDROID_KEY_ALIAS' }
if ([string]::IsNullOrWhiteSpace($secretKeyPassword)) { $missing += 'AI_TERMINAL_ANDROID_KEY_PASSWORD' }
if ($missing.Count -gt 0) {
  throw "Missing Android GitHub signing secret environment variable(s): $($missing -join ', ')"
}

$decodedKeystore = Join-Path $resolvedSmokeRoot 'decoded-github-secret-keystore.jks'
try {
  $normalizedBase64 = ($secretKeystoreBase64 -replace '\s+', '')
  [System.IO.File]::WriteAllBytes($decodedKeystore, [Convert]::FromBase64String($normalizedBase64))
} catch {
  throw "AI_TERMINAL_ANDROID_KEYSTORE_BASE64 is not valid base64: $($_.Exception.Message)"
}

$keytoolListOutput = & $keytool.Source `
  -list `
  -v `
  -keystore $decodedKeystore `
  -storepass $secretKeystorePassword `
  -alias $secretKeyAlias 2>&1
if ($LASTEXITCODE -ne 0) {
  $keytoolListOutput | Set-Content -LiteralPath (Join-Path $resolvedSmokeRoot 'keytool-list.txt') -Encoding utf8
  throw "Decoded keystore could not be opened with the provided alias/passwords"
}

$env:ANDROID_HOME = $AndroidHome
$env:ANDROID_SDK_ROOT = $AndroidHome
$env:AI_TERMINAL_ANDROID_KEYSTORE = $decodedKeystore
$env:AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD = $secretKeystorePassword
$env:AI_TERMINAL_ANDROID_KEY_ALIAS = $secretKeyAlias
$env:AI_TERMINAL_ANDROID_KEY_PASSWORD = $secretKeyPassword

Push-Location $androidDir
try {
  & .\gradlew.bat :app:verifyFdroidReleaseInputs :app:assembleRelease :app:verifyNativeLibraries
  if ($LASTEXITCODE -ne 0) {
    throw "Gradle GitHub signing preflight failed with exit code $LASTEXITCODE"
  }
} finally {
  Pop-Location
}

$signedApk = Join-Path $androidDir 'app\build\outputs\apk\release\app-release.apk'
if (-not (Test-Path -LiteralPath $signedApk -PathType Leaf)) {
  throw "signed release APK was not generated: $signedApk"
}

$verifyOutput = & $apksigner verify --verbose --print-certs $signedApk 2>&1
if ($LASTEXITCODE -ne 0) {
  $verifyOutput | Set-Content -LiteralPath (Join-Path $resolvedSmokeRoot 'apksigner-verify.txt') -Encoding utf8
  throw "apksigner verify failed; see GitHub signing preflight evidence"
}

$badging = & $aapt dump badging $signedApk
$packageLine = ($badging | Select-String -Pattern '^package:' | Select-Object -First 1).Line
$apkItem = Get-Item -LiteralPath $signedApk
$apkHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $signedApk).Hash.ToLowerInvariant()
$keystoreHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $decodedKeystore).Hash.ToLowerInvariant()
$certSha256 = ($verifyOutput | Select-String -Pattern 'Signer #1 certificate SHA-256 digest:' | Select-Object -First 1).Line

$evidence = [pscustomobject]@{
  status = 'passed'
  timestamp = (Get-Date).ToString('o')
  mode = $mode
  signedApk = $signedApk
  apkSize = $apkItem.Length
  apkSha256 = $apkHash
  package = $packageLine
  decodedKeystoreSha256 = $keystoreHash
  keyAlias = $secretKeyAlias
  certificate = $certSha256
  apksigner = $verifyOutput
  note = 'Passwords and base64 keystore secret are intentionally omitted.'
}
$evidence | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
Write-Output "ANDROID_GITHUB_SIGNING_PREFLIGHT_OK $EvidencePath"
