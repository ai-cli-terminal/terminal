# Human-readable status for the v0.3.3 release follow-up evidence.
#
# By default this reads the latest combined preflight evidence. Use -Refresh to
# rerun scripts/smoke-release-followup-preflight.ps1 first.
param(
  [string]$EvidencePath = '',
  [switch]$Refresh,
  [switch]$RunMsiBuild,
  [switch]$RunAndroidLocalSmokes,
  [string]$FdroidBuildEvidencePath = '',
  [switch]$Json,
  [switch]$FailOnBlocked
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
$evidenceRoot = Join-Path $repoRoot 'artifacts\release-followup-preflight'
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $evidenceRoot 'release-followup-preflight-evidence.json'
}

function Invoke-ReleaseFollowupPreflight {
  $args = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', (Join-Path $repoRoot 'scripts\smoke-release-followup-preflight.ps1'),
    '-EvidencePath', $EvidencePath
  )
  if ($RunMsiBuild) {
    $args += '-RunMsiBuild'
  }
  if ($RunAndroidLocalSmokes) {
    $args += '-RunAndroidLocalSmokes'
  }
  if (-not [string]::IsNullOrWhiteSpace($FdroidBuildEvidencePath)) {
    $args += @('-FdroidBuildEvidencePath', $FdroidBuildEvidencePath)
  }

  & pwsh @args
  if ($LASTEXITCODE -ne 0) {
    throw "release follow-up preflight failed with exit code $LASTEXITCODE"
  }
}

function Get-NonEmptyArray {
  param(
    [object]$Value
  )

  @($Value | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
}

if ($Refresh -or -not (Test-Path -LiteralPath $EvidencePath -PathType Leaf)) {
  New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
  Invoke-ReleaseFollowupPreflight
}

if (-not (Test-Path -LiteralPath $EvidencePath -PathType Leaf)) {
  throw "release follow-up evidence not found: $EvidencePath"
}

$evidence = Get-Content -Raw -LiteralPath $EvidencePath | ConvertFrom-Json
if (-not $evidence.closeout) {
  throw "release follow-up evidence does not include closeout; rerun npm run smoke:release-followup-preflight"
}

$closeout = $evidence.closeout
$items = @($closeout.items)
$blockedItems = @($items | Where-Object { $_.status -ne 'ready' })
$readyItems = @($items | Where-Object { $_.status -eq 'ready' })
$blockers = Get-NonEmptyArray $evidence.blockers
$nextActions = Get-NonEmptyArray $evidence.nextActions
$timestamp = if ($evidence.timestamp -is [datetime]) {
  $evidence.timestamp.ToString('o')
} else {
  [string]$evidence.timestamp
}
$summary = [pscustomobject]@{
  status = $evidence.status
  canCloseDocs = [bool]$closeout.canCloseDocs
  evidencePath = [System.IO.Path]::GetFullPath($EvidencePath)
  timestamp = $timestamp
  readyItems = @($readyItems | ForEach-Object { $_.name })
  blockedItems = @($blockedItems | ForEach-Object { $_.name })
  releaseTagAction = $closeout.releaseTagAction
  assetAction = $closeout.assetAction
  blockers = $blockers
  nextActions = $nextActions
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 6
} else {
  Write-Output "RELEASE_FOLLOWUP_STATUS $($summary.status)"
  Write-Output "Evidence: $($summary.evidencePath)"
  Write-Output "Timestamp: $($summary.timestamp)"
  Write-Output "Can close docs: $($summary.canCloseDocs)"
  Write-Output "Release tag action: $($summary.releaseTagAction)"
  Write-Output "Asset action: $($summary.assetAction)"
  Write-Output ''

  if ($readyItems.Count -gt 0) {
    Write-Output 'Ready items:'
    foreach ($item in $readyItems) {
      Write-Output "- $($item.name)"
    }
    Write-Output ''
  }

  if ($blockedItems.Count -gt 0) {
    Write-Output 'Blocked items:'
    foreach ($item in $blockedItems) {
      Write-Output "- $($item.name): $($item.note)"
      $missing = Get-NonEmptyArray $item.missing
      if ($missing.Count -gt 0) {
        Write-Output "  missing: $($missing -join ', ')"
      }
      if (-not [string]::IsNullOrWhiteSpace($item.evidencePath)) {
        Write-Output "  evidence: $($item.evidencePath)"
      }
    }
    Write-Output ''
  }

  if ($blockers.Count -gt 0) {
    Write-Output 'Blockers:'
    foreach ($blocker in $blockers) {
      Write-Output "- $blocker"
    }
    Write-Output ''
  }

  if ($nextActions.Count -gt 0) {
    Write-Output 'Next actions:'
    for ($i = 0; $i -lt $nextActions.Count; $i += 1) {
      Write-Output "$($i + 1). $($nextActions[$i])"
    }
  }
}

if ($FailOnBlocked -and -not $summary.canCloseDocs) {
  exit 2
}
