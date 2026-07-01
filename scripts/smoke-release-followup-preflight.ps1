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

function Get-FdroidBuildEvidence {
  param([string]$Path)

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return [pscustomobject]@{
      status = 'blocked'
      evidencePath = $null
      note = 'No fdroid build/buildserver evidence path supplied'
    }
  }

  $resolved = [System.IO.Path]::GetFullPath($Path)
  if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
    return [pscustomobject]@{
      status = 'blocked'
      evidencePath = $resolved
      note = 'Supplied fdroid build/buildserver evidence path does not exist'
    }
  }

  [pscustomobject]@{
    status = 'ready'
    evidencePath = $resolved
    note = 'Caller supplied an existing fdroid build/buildserver evidence file'
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
$fdroidBuild = Get-FdroidBuildEvidence -Path $FdroidBuildEvidencePath

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
  $blockers += 'F-Droid build/buildserver evidence is not supplied'
}

$status = if ($blockers.Count -eq 0) { 'ready' } else { 'blocked' }
$evidence = [pscustomobject]@{
  status = $status
  timestamp = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  msi = $msi
  androidSigningSecrets = $androidSecrets
  fdroidBuild = $fdroidBuild
  androidLocalSmokes = $androidLocalSmokes
  blockers = $blockers
  nextActions = @(
    'Run scripts/smoke-msi-preflight.ps1 -RunBuild on a Windows-native Rust/MSVC/WiX host',
    'Register the four AI_TERMINAL_ANDROID_* GitHub release signing secrets',
    'Capture fdroid build/buildserver evidence and pass its path with -FdroidBuildEvidencePath'
  )
}

$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

if ($status -eq 'ready') {
  Write-Output "RELEASE_FOLLOWUP_PREFLIGHT_READY $EvidencePath"
} else {
  Write-Output "RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED $EvidencePath"
}
