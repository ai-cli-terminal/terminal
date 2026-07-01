# Operator-facing release follow-up check.
#
# Runs the deterministic status smoke, refreshes combined preflight evidence,
# then prints/saves the current release follow-up status summary.
param(
  [string]$EvidencePath = '',
  [string]$CheckEvidencePath = '',
  [switch]$RunMsiBuild,
  [switch]$RunAndroidLocalSmokes,
  [string]$FdroidBuildEvidencePath = '',
  [switch]$Json,
  [switch]$FailOnBlocked
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
$preflightRoot = Join-Path $repoRoot 'artifacts\release-followup-preflight'
$checkRoot = Join-Path $repoRoot 'artifacts\release-followup-check'
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $preflightRoot 'release-followup-preflight-evidence.json'
}
if ([string]::IsNullOrWhiteSpace($CheckEvidencePath)) {
  $CheckEvidencePath = Join-Path $checkRoot 'release-followup-check-evidence.json'
}
New-Item -ItemType Directory -Force -Path $preflightRoot | Out-Null
New-Item -ItemType Directory -Force -Path $checkRoot | Out-Null

function Invoke-CheckStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Script
  )

  $started = Get-Date
  $global:LASTEXITCODE = 0
  try {
    $output = @(& $Script 2>&1)
    $exitCode = if ($null -ne $global:LASTEXITCODE) { [int]$global:LASTEXITCODE } else { 0 }
    [pscustomobject]@{
      name = $Name
      status = if ($exitCode -eq 0) { 'passed' } else { 'failed' }
      exitCode = $exitCode
      startedAt = $started.ToString('o')
      finishedAt = (Get-Date).ToString('o')
      outputTail = @($output | ForEach-Object { "$_" } | Select-Object -Last 80)
    }
  } catch {
    [pscustomobject]@{
      name = $Name
      status = 'failed'
      exitCode = 1
      startedAt = $started.ToString('o')
      finishedAt = (Get-Date).ToString('o')
      outputTail = @($_.Exception.Message)
    }
  } finally {
    $global:LASTEXITCODE = 0
  }
}

$statusSmokeEvidencePath = Join-Path $checkRoot 'release-followup-status-smoke-evidence.json'
$statusSmokeRoot = Join-Path $checkRoot 'status-smoke'
$steps = @()
$steps += Invoke-CheckStep -Name 'status-smoke' -Script {
  & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot 'scripts\smoke-release-followup-status.ps1') `
    -SmokeRoot $statusSmokeRoot `
    -EvidencePath $statusSmokeEvidencePath
}

$preflightArgs = @(
  '-NoProfile',
  '-ExecutionPolicy', 'Bypass',
  '-File', (Join-Path $repoRoot 'scripts\smoke-release-followup-preflight.ps1'),
  '-EvidencePath', $EvidencePath
)
if ($RunMsiBuild) {
  $preflightArgs += '-RunMsiBuild'
}
if ($RunAndroidLocalSmokes) {
  $preflightArgs += '-RunAndroidLocalSmokes'
}
if (-not [string]::IsNullOrWhiteSpace($FdroidBuildEvidencePath)) {
  $preflightArgs += @('-FdroidBuildEvidencePath', $FdroidBuildEvidencePath)
}
$steps += Invoke-CheckStep -Name 'combined-preflight' -Script {
  & pwsh @preflightArgs
}

$statusJsonPath = Join-Path $checkRoot 'release-followup-status-summary.json'
$statusStep = Invoke-CheckStep -Name 'status-summary-json' -Script {
  $summaryJson = & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot 'scripts\show-release-followup-status.ps1') `
    -EvidencePath $EvidencePath `
    -Json
  $summaryJson | Set-Content -LiteralPath $statusJsonPath -Encoding utf8
  $summaryJson
}
$steps += $statusStep

$summary = $null
if ($statusStep.status -eq 'passed' -and (Test-Path -LiteralPath $statusJsonPath -PathType Leaf)) {
  $summary = Get-Content -Raw -LiteralPath $statusJsonPath | ConvertFrom-Json
}

$failedSteps = @($steps | Where-Object { $_.status -ne 'passed' })
$overall = if ($failedSteps.Count -gt 0) {
  'failed'
} elseif ($summary -and [bool]$summary.canCloseDocs) {
  'ready'
} else {
  'blocked'
}

$checkEvidence = [pscustomobject]@{
  status = $overall
  timestamp = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  evidencePath = [System.IO.Path]::GetFullPath($EvidencePath)
  checkEvidencePath = [System.IO.Path]::GetFullPath($CheckEvidencePath)
  statusSmokeEvidencePath = [System.IO.Path]::GetFullPath($statusSmokeEvidencePath)
  statusSummaryPath = [System.IO.Path]::GetFullPath($statusJsonPath)
  steps = $steps
  summary = $summary
}
$checkEvidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $CheckEvidencePath -Encoding utf8

if ($Json) {
  $checkEvidence | ConvertTo-Json -Depth 8
} else {
  foreach ($step in @($steps | Where-Object { $_.name -ne 'status-summary-json' })) {
    foreach ($line in @($step.outputTail)) {
      Write-Output $line
    }
  }

  if ($summary) {
    Write-Output ''
    Write-Output "Release follow-up check: $overall"
    Write-Output "Can close docs: $($summary.canCloseDocs)"
    Write-Output "Blocked items: $(@($summary.blockedItems) -join ', ')"
    Write-Output "Check evidence: $CheckEvidencePath"
  } else {
    Write-Output ''
    Write-Output "Release follow-up check: $overall"
    Write-Output "Check evidence: $CheckEvidencePath"
  }
}

if ($overall -eq 'failed') {
  exit 1
}
if ($FailOnBlocked -and $overall -ne 'ready') {
  exit 2
}
