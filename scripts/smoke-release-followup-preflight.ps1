# Release follow-up preflight for the remaining v0.3.3 blockers.
#
# This script intentionally does not print or persist secret values. It records:
# - Windows MSI packaging readiness through scripts/smoke-msi-preflight.ps1
# - presence of the GitHub Android signing secret names and workflow references
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

function Get-AndroidSigningExpectations {
  $required = @(
    'AI_TERMINAL_ANDROID_KEYSTORE_BASE64',
    'AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD',
    'AI_TERMINAL_ANDROID_KEY_ALIAS',
    'AI_TERMINAL_ANDROID_KEY_PASSWORD'
  )
  [pscustomobject]@{
    required = $required
    workflowPath = Join-Path $repoRoot '.github\workflows\release.yml'
  }
}

function Get-AndroidSigningWorkflowReferences {
  param(
    [Parameter(Mandatory = $true)]
    [pscustomobject]$Expected
  )

  $workflowPath = [System.IO.Path]::GetFullPath($Expected.workflowPath)
  if (-not (Test-Path -LiteralPath $workflowPath -PathType Leaf)) {
    return [pscustomobject]@{
      status = 'blocked'
      path = $workflowPath
      referenced = @()
      missing = @($Expected.required)
      note = 'Release workflow file is missing'
    }
  }

  $workflow = Get-Content -Raw -LiteralPath $workflowPath
  $referenced = @($Expected.required | Where-Object {
    $workflow -match "secrets\s*\.\s*$([regex]::Escape($_))\b"
  })
  $missing = @($Expected.required | Where-Object { $referenced -notcontains $_ })

  [pscustomobject]@{
    status = if ($missing.Count -eq 0) { 'ready' } else { 'blocked' }
    path = $workflowPath
    referenced = $referenced
    missing = $missing
    note = 'Checks release workflow references only; secret values are never read'
  }
}

function Get-GitHubAndroidSecrets {
  $expected = Get-AndroidSigningExpectations
  $required = @($expected.required)
  $workflow = Get-AndroidSigningWorkflowReferences -Expected $expected

  $gh = Get-Command gh -ErrorAction SilentlyContinue
  if (-not $gh) {
    return [pscustomobject]@{
      status = 'blocked'
      required = $required
      workflow = $workflow
      present = @()
      presentDetails = @()
      missing = $required
      note = 'gh CLI is not available; cannot check repository secret names'
    }
  }

  $raw = & $gh.Source secret list --json name,updatedAt 2>&1
  if ($LASTEXITCODE -ne 0) {
    return [pscustomobject]@{
      status = 'blocked'
      required = $required
      workflow = $workflow
      present = @()
      presentDetails = @()
      missing = $required
      note = "gh secret list failed: $($raw -join "`n")"
    }
  }

  $rawText = ($raw | Out-String).Trim()
  $secrets = if ([string]::IsNullOrWhiteSpace($rawText)) { @() } else { @($rawText | ConvertFrom-Json) }
  $secretNames = @($secrets | ForEach-Object { $_.name })
  $present = @($required | Where-Object { $secretNames -contains $_ })
  $missing = @($required | Where-Object { $secretNames -notcontains $_ })
  $presentDetails = @($secrets | Where-Object { $required -contains $_.name } | ForEach-Object {
    [pscustomobject]@{
      name = $_.name
      updatedAt = $_.updatedAt
    }
  })

  [pscustomobject]@{
    status = if ($missing.Count -eq 0 -and $workflow.status -eq 'ready') { 'ready' } else { 'blocked' }
    required = $required
    workflow = $workflow
    present = $present
    presentDetails = $presentDetails
    missing = $missing
    note = 'Only secret names and workflow references are recorded; secret values are never read or written'
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
    checks = if ($payload) { $payload.checks } else { $null }
    build = if ($payload) { $payload.build } else { $null }
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
} elseif (-not $RunMsiBuild) {
  $blockers += 'Windows MSI build evidence was not requested; rerun with -RunMsiBuild on a Windows-native Rust/MSVC/WiX host'
}
if ($androidSecrets.workflow.status -ne 'ready') {
  $blockers += "Release workflow is missing Android signing secret reference(s): $(@($androidSecrets.workflow.missing) -join ', ')"
}
if (@($androidSecrets.missing).Count -gt 0) {
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

$msiCloseoutMissing = @()
if ($msi.status -ne 'ready') {
  $msiCloseoutMissing += @($msi.missing)
  if ($msiCloseoutMissing.Count -eq 0) {
    $msiCloseoutMissing += "MSI status $($msi.status)"
  }
}
if (-not $RunMsiBuild) {
  $msiCloseoutMissing += 'RunMsiBuild evidence'
}
$msiCloseoutStatus = if ($msi.status -eq 'ready' -and $RunMsiBuild) { 'ready' } else { 'blocked' }

$androidCloseoutMissing = @()
if ($androidSecrets.workflow.status -ne 'ready') {
  $androidCloseoutMissing += @($androidSecrets.workflow.missing | ForEach-Object { "workflow reference $_" })
}
if (@($androidSecrets.missing).Count -gt 0) {
  $androidCloseoutMissing += @($androidSecrets.missing | ForEach-Object { "repository secret name $_" })
}
if ($androidSecrets.status -ne 'ready' -and $androidCloseoutMissing.Count -eq 0) {
  $androidCloseoutMissing += "Android signing status $($androidSecrets.status)"
}
$androidCloseoutStatus = if ($androidSecrets.status -eq 'ready') { 'ready' } else { 'blocked' }

$fdroidCloseoutMissing = @($fdroidBuild.missing)
$fdroidCloseoutStatus = if ($fdroidBuild.status -eq 'ready') { 'ready' } else { 'blocked' }
if ($fdroidCloseoutStatus -ne 'ready' -and $fdroidCloseoutMissing.Count -eq 0) {
  $fdroidCloseoutMissing += $fdroidBuild.note
}

$closeoutItems = @(
  [pscustomobject]@{
    name = 'msi'
    status = $msiCloseoutStatus
    evidencePath = $msi.evidencePath
    required = @('RunMsiBuild', 'successful build command', 'generated MSI path', 'SHA256 hash')
    missing = $msiCloseoutMissing
    note = if ($msiCloseoutStatus -eq 'ready') {
      'MSI build evidence is ready'
    } else {
      'MSI closeout requires -RunMsiBuild evidence from a Windows-native Rust/MSVC/WiX host'
    }
  }
  [pscustomobject]@{
    name = 'androidSigningSecrets'
    status = $androidCloseoutStatus
    evidencePath = $null
    required = @('workflow references', 'repository secret names')
    missing = $androidCloseoutMissing
    note = if ($androidCloseoutStatus -eq 'ready') {
      'Android signing secret-name evidence is ready; secret values were not read'
    } else {
      'Android signing closeout requires workflow references and repository secret names'
    }
  }
  [pscustomobject]@{
    name = 'fdroidBuild'
    status = $fdroidCloseoutStatus
    evidencePath = $fdroidBuild.evidencePath
    required = @('evidence file', 'app id', 'versionName', 'versionCode', 'successful result', 'APK or buildserver artifact')
    missing = $fdroidCloseoutMissing
    note = if ($fdroidCloseoutStatus -eq 'ready') {
      'F-Droid build/buildserver evidence is ready'
    } else {
      'F-Droid closeout requires supplied build/buildserver evidence for the expected app and version'
    }
  }
)
$closeoutReadyItems = @($closeoutItems | Where-Object { $_.status -eq 'ready' } | ForEach-Object { $_.name })
$closeoutBlockedItems = @($closeoutItems | Where-Object { $_.status -ne 'ready' } | ForEach-Object { $_.name })
$canCloseDocs = ($status -eq 'ready' -and $closeoutBlockedItems.Count -eq 0)
$closeout = [pscustomobject]@{
  status = $status
  ready = $canCloseDocs
  canCloseDocs = $canCloseDocs
  requiredEvidence = @('msi', 'androidSigningSecrets', 'fdroidBuild')
  readyItems = $closeoutReadyItems
  blockedItems = $closeoutBlockedItems
  items = $closeoutItems
  releaseTagAction = 'unchanged'
  assetAction = 'unchanged'
  note = if ($canCloseDocs) {
    'All release follow-up evidence gates are ready; close follow-up docs without changing the existing v0.3.3 tag/assets unless a separate release decision says otherwise'
  } else {
    'Do not mark the release follow-up closed while closeout.blockedItems is non-empty'
  }
}
$evidence = [pscustomobject]@{
  status = $status
  timestamp = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  msi = $msi
  androidSigningSecrets = $androidSecrets
  fdroidExpectations = $fdroidExpected
  fdroidBuild = $fdroidBuild
  androidLocalSmokes = $androidLocalSmokes
  closeout = $closeout
  blockers = $blockers
  nextActions = @(
    'Run scripts/smoke-msi-preflight.ps1 -RunBuild on a Windows-native Rust/MSVC/WiX host',
    'Register the four AI_TERMINAL_ANDROID_* GitHub release signing secrets referenced by .github/workflows/release.yml',
    "Capture fdroid build/buildserver evidence for $($fdroidExpected.appId) $($fdroidExpected.versionName) ($($fdroidExpected.versionCode)) and pass its path with -FdroidBuildEvidencePath"
  )
}

$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

if ($status -eq 'ready') {
  Write-Output "RELEASE_FOLLOWUP_PREFLIGHT_READY $EvidencePath"
} else {
  Write-Output "RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED $EvidencePath"
}
