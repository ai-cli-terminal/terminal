# Release follow-up preflight for the remaining v0.3.3 blockers.
#
# This script intentionally does not print or persist secret values. It records:
# - Windows MSI packaging readiness through scripts/smoke-msi-preflight.ps1
# - presence of the GitHub Android signing secret names
# - whether F-Droid build/buildserver evidence has been supplied
#
# Optional -RunAndroidLocalSmokes reruns local throwaway signing and F-Droid
# metadata smokes. Those prove wiring, not real release signing/buildserver.
param(
  [string]$EvidencePath = '',
  [switch]$RunMsiBuild,
  [switch]$RunAndroidLocalSmokes,
  [string]$FdroidBuildEvidencePath = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
$evidenceRoot = Join-Path $repoRoot 'artifacts\release-followup-preflight'
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $evidenceRoot 'release-followup-preflight-evidence.json'
}
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null

function Invoke-Step {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Script
  )

  try {
    & $Script
  } catch {
    [pscustomobject]@{
      status = 'error'
      name = $Name
      error = $_.Exception.Message
    }
  }
}

function Get-GitHubAndroidSecrets {
  $required = @(
    'AI_TERMINAL_ANDROID_KEYSTORE_BASE64',
    'AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD',
    'AI_TERMINAL_ANDROID_KEY_ALIAS',
    'AI_TERMINAL_ANDROID_KEY_PASSWORD'
  )

  $gh = Get-Command gh -ErrorAction SilentlyContinue
  if (-not $gh) {
    return [pscustomobject]@{
      status = 'blocked'
      required = $required
      present = @()
      missing = $required
      note = 'gh CLI is not available; cannot check repository secret names'
    }
  }

  $raw = & $gh.Source secret list --json name,updatedAt 2>&1
  if ($LASTEXITCODE -ne 0) {
    return [pscustomobject]@{
      status = 'blocked'
      required = $required
      present = @()
      missing = $required
      note = "gh secret list failed: $($raw -join "`n")"
    }
  }

  $rawText = ($raw | Out-String).Trim()
  $secrets = if ([string]::IsNullOrWhiteSpace($rawText)) { @() } else { @($rawText | ConvertFrom-Json) }
  $secretNames = @($secrets | ForEach-Object { $_.name })
  $present = @($required | Where-Object { $secretNames -contains $_ })
  $missing = @($required | Where-Object { $secretNames -notcontains $_ })

  [pscustomobject]@{
    status = if ($missing.Count -eq 0) { 'ready' } else { 'blocked' }
    required = $required
    present = $present
    missing = $missing
    note = 'Only secret names are recorded; secret values are never read or written'
  }
}

function Get-FdroidExpectations {
  $versionPath = Join-Path $repoRoot 'android\fdroid-version.properties'
  if (-not (Test-Path -LiteralPath $versionPath -PathType Leaf)) {
    throw "F-Droid version properties not found: $versionPath"
  }

  $versionProperties = Get-Content -Raw -LiteralPath $versionPath | ConvertFrom-StringData
  if ([string]::IsNullOrWhiteSpace($versionProperties.versionName) -or [string]::IsNullOrWhiteSpace($versionProperties.versionCode)) {
    throw "F-Droid version properties must define versionName and versionCode: $versionPath"
  }

  [pscustomobject]@{
    appId = 'dev.aiterminal.android'
    versionName = [string]$versionProperties.versionName
    versionCode = [string]$versionProperties.versionCode
    versionPath = $versionPath
  }
}

function Get-FdroidBuildEvidence {
  param(
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [pscustomobject]$Expected
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return [pscustomobject]@{
      status = 'blocked'
      evidencePath = $null
      expected = $Expected
      missing = @('evidencePath')
      checks = $null
      note = 'No fdroid build/buildserver evidence path supplied'
    }
  }

  $resolved = [System.IO.Path]::GetFullPath($Path)
  if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
    return [pscustomobject]@{
      status = 'blocked'
      evidencePath = $resolved
      expected = $Expected
      missing = @('existing evidence file')
      checks = $null
      note = 'Supplied fdroid build/buildserver evidence path does not exist'
    }
  }

  $raw = Get-Content -Raw -LiteralPath $resolved
  $jsonParsed = $false
  if (-not [string]::IsNullOrWhiteSpace($raw)) {
    try {
      $null = $raw | ConvertFrom-Json -ErrorAction Stop
      $jsonParsed = $true
    } catch {
      $jsonParsed = $false
    }
  }

  $versionCodePattern = "(?<!\d)$([regex]::Escape($Expected.versionCode))(?!\d)"
  $resultPattern = '(?i)\b(status|result)\b[^\r\n]*(ready|passed|success|succeeded|built|ok)\b|\b(build|fdroid|buildserver)\b[^\r\n]*(ready|passed|success|succeeded|built|ok)\b'
  $artifactPattern = '(?i)(\.apk\b|artifact|output|buildserver)'
  $checks = [pscustomobject]@{
    nonEmpty = -not [string]::IsNullOrWhiteSpace($raw)
    jsonParsed = $jsonParsed
    appId = $raw.Contains($Expected.appId)
    versionName = $raw.Contains($Expected.versionName)
    versionCode = $raw -match $versionCodePattern
    resultStatus = $raw -match $resultPattern
    outputArtifact = $raw -match $artifactPattern
  }

  $missing = @()
  if (-not $checks.nonEmpty) { $missing += 'non-empty evidence content' }
  if (-not $checks.appId) { $missing += "app id $($Expected.appId)" }
  if (-not $checks.versionName) { $missing += "versionName $($Expected.versionName)" }
  if (-not $checks.versionCode) { $missing += "versionCode $($Expected.versionCode)" }
  if (-not $checks.resultStatus) { $missing += 'successful build result/status' }
  if (-not $checks.outputArtifact) { $missing += 'APK output or buildserver artifact reference' }

  [pscustomobject]@{
    status = if ($missing.Count -eq 0) { 'ready' } else { 'blocked' }
    evidencePath = $resolved
    expected = $Expected
    missing = $missing
    checks = $checks
    note = if ($missing.Count -eq 0) {
      'Caller supplied build/buildserver evidence matching the expected F-Droid app and version'
    } else {
      'Supplied fdroid build/buildserver evidence is missing required release markers'
    }
  }
}

$msiEvidencePath = Join-Path $evidenceRoot 'msi-preflight-evidence.json'
$msi = Invoke-Step -Name 'msi-preflight' -Script {
  $args = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', (Join-Path $repoRoot 'scripts\smoke-msi-preflight.ps1'),
    '-EvidencePath', $msiEvidencePath
  )
  if ($RunMsiBuild) {
    $args += '-RunBuild'
  }

  $output = & pwsh @args 2>&1
  $exitCode = $LASTEXITCODE
  $payload = if (Test-Path -LiteralPath $msiEvidencePath -PathType Leaf) {
    Get-Content -Raw -LiteralPath $msiEvidencePath | ConvertFrom-Json
  } else {
    $null
  }

  [pscustomobject]@{
    status = if ($payload) { $payload.status } else { 'error' }
    exitCode = $exitCode
    output = @($output)
    evidencePath = $msiEvidencePath
    missing = if ($payload) { @($payload.missing) } else { @() }
  }
}

$androidSecrets = Get-GitHubAndroidSecrets
$fdroidExpected = Get-FdroidExpectations
$fdroidBuild = Get-FdroidBuildEvidence -Path $FdroidBuildEvidencePath -Expected $fdroidExpected

$androidLocalSmokes = [pscustomobject]@{
  attempted = $false
  results = @()
}
if ($RunAndroidLocalSmokes) {
  $results = @()
  $results += Invoke-Step -Name 'android-throwaway-signing' -Script {
    $smokeRoot = Join-Path $evidenceRoot 'android-github-signing-preflight'
    $evidence = Join-Path $smokeRoot 'android-github-signing-preflight-evidence.json'
    $output = & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot 'android\smoke-github-signing-secrets.ps1') -UseThrowawayKeystore -SmokeRoot $smokeRoot -EvidencePath $evidence 2>&1
    [pscustomobject]@{
      status = if ($LASTEXITCODE -eq 0) { 'passed' } else { 'failed' }
      output = @($output)
      evidencePath = $evidence
    }
  }
  $results += Invoke-Step -Name 'fdroid-metadata' -Script {
    $smokeRoot = Join-Path $evidenceRoot 'fdroid-dry-run'
    $output = & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot 'android\smoke-fdroid-metadata.ps1') -SmokeRoot $smokeRoot 2>&1
    [pscustomobject]@{
      status = if ($LASTEXITCODE -eq 0) { 'passed' } else { 'failed' }
      output = @($output)
      evidencePath = Join-Path $smokeRoot 'fdroid-metadata-smoke-evidence.json'
    }
  }

  $androidLocalSmokes = [pscustomobject]@{
    attempted = $true
    results = $results
  }
}

$blockers = @()
if ($msi.status -ne 'ready') {
  $blockers += "Windows MSI toolchain/build not ready: $(@($msi.missing) -join ', ')"
}
if ($androidSecrets.status -ne 'ready') {
  $blockers += "Missing GitHub Android signing secret name(s): $(@($androidSecrets.missing) -join ', ')"
}
if ($fdroidBuild.status -ne 'ready') {
  $missingFdroid = @($fdroidBuild.missing) -join ', '
  if ([string]::IsNullOrWhiteSpace($missingFdroid)) {
    $missingFdroid = $fdroidBuild.note
  }
  $blockers += "F-Droid build/buildserver evidence is not ready: $missingFdroid"
}

$status = if ($blockers.Count -eq 0) { 'ready' } else { 'blocked' }
$evidence = [pscustomobject]@{
  status = $status
  timestamp = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  msi = $msi
  androidSigningSecrets = $androidSecrets
  fdroidExpectations = $fdroidExpected
  fdroidBuild = $fdroidBuild
  androidLocalSmokes = $androidLocalSmokes
  blockers = $blockers
  nextActions = @(
    'Run scripts/smoke-msi-preflight.ps1 -RunBuild on a Windows-native Rust/MSVC/WiX host',
    'Register the four AI_TERMINAL_ANDROID_* GitHub release signing secrets',
    "Capture fdroid build/buildserver evidence for $($fdroidExpected.appId) $($fdroidExpected.versionName) ($($fdroidExpected.versionCode)) and pass its path with -FdroidBuildEvidencePath"
  )
}

$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

if ($status -eq 'ready') {
  Write-Output "RELEASE_FOLLOWUP_PREFLIGHT_READY $EvidencePath"
} else {
  Write-Output "RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED $EvidencePath"
}
