# RA/PWA live approval evidence smoke.
# Usage:
#   pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1
#   pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1 -SkipRust
param(
  [string]$EvidencePath = '',
  [switch]$SkipRust
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
$evidenceRoot = Join-Path $repoRoot 'artifacts\ra-pwa-live-evidence'
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $evidenceRoot 'ra-pwa-live-evidence.json'
}
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null

function Invoke-EvidenceStep {
  param(
    [string]$Name,
    [scriptblock]$Script,
    [switch]$AllowBlocked
  )

  $started = Get-Date
  try {
    $output = @(& $Script 2>&1)
    $exitCode = if ($null -ne $global:LASTEXITCODE) { [int]$global:LASTEXITCODE } else { 0 }
    $status = if ($exitCode -eq 0) { 'passed' } else { 'failed' }
    if ($AllowBlocked -and $exitCode -eq 88) {
      $status = 'blocked'
    }
    [pscustomobject]@{
      name = $Name
      status = $status
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

function Test-PwaLiveSelectors {
  $index = Get-Content -LiteralPath (Join-Path $repoRoot 'pwa\index.html') -Raw
  $app = Get-Content -LiteralPath (Join-Path $repoRoot 'pwa\app.mjs') -Raw
  $required = @(
    'id="live-endpoint"',
    'id="live-connect-button"',
    'id="live-disconnect-button"',
    'id="live-state"',
    'id="monitor-state"',
    'id="monitor-event-log"',
    'id="live-approval-list"',
    'function connectLive',
    'new window.EventSource',
    'postLiveTransportMessage(liveBaseUrl, liveApprovalResponseMessage(response))'
  )
  $missing = @()
  foreach ($needle in $required) {
    if (-not ($index.Contains($needle) -or $app.Contains($needle))) {
      $missing += $needle
    }
  }
  if ($missing.Count -gt 0) {
    throw "missing PWA live selector/helper surface: $($missing -join ', ')"
  }
  "PWA_LIVE_SELECTORS_OK"
}

Push-Location $repoRoot
try {
  $steps = @()
  $steps += Invoke-EvidenceStep -Name 'pwa-node-live-helper-tests' -Script {
    & node 'pwa/app.test.mjs'
  }
  $steps += Invoke-EvidenceStep -Name 'pwa-live-selector-surface' -Script {
    Test-PwaLiveSelectors
  }
  if ($SkipRust) {
    $steps += [pscustomobject]@{
      name = 'rust-remote-live-bridge-tests'
      status = 'skipped'
      exitCode = 0
      startedAt = (Get-Date).ToString('o')
      finishedAt = (Get-Date).ToString('o')
      outputTail = @('SkipRust requested')
    }
  } else {
    $steps += Invoke-EvidenceStep -Name 'rust-remote-live-bridge-tests' -AllowBlocked -Script {
      $wsl = Get-Command 'wsl.exe' -ErrorAction SilentlyContinue
      if (-not $wsl) {
        Write-Output 'wsl.exe not found'
        $global:LASTEXITCODE = 88
        return
      }
      $env:MSYS_NO_PATHCONV = '1'
      & wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo test --features remote companion_live'
    }
  }

  $failed = @($steps | Where-Object { $_.status -eq 'failed' })
  $blocked = @($steps | Where-Object { $_.status -eq 'blocked' })
  $overall = if ($failed.Count -gt 0) {
    'failed'
  } elseif ($blocked.Count -gt 0) {
    'blocked'
  } else {
    'passed'
  }

  $evidence = [pscustomobject]@{
    status = $overall
    timestamp = (Get-Date).ToString('o')
    repoRoot = $repoRoot
    evidencePath = $EvidencePath
    pwaFiles = @(
      'pwa/index.html',
      'pwa/app.mjs',
      'pwa/app.test.mjs',
      'pwa/styles.css'
    )
    steps = $steps
    nextEvidence = @(
      'start ai remote daemon --device-id <id>',
      'connect browser/PWA to printed live endpoint',
      'run ai remote arm --allow-high',
      'approve and reject High-risk commands from the PWA'
    )
  }
  $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

  if ($overall -eq 'passed') {
    Write-Output "RA_PWA_LIVE_EVIDENCE_OK $EvidencePath"
    exit 0
  }
  if ($overall -eq 'blocked') {
    Write-Output "RA_PWA_LIVE_EVIDENCE_BLOCKED $EvidencePath"
    exit 2
  }
  Write-Output "RA_PWA_LIVE_EVIDENCE_FAILED $EvidencePath"
  exit 1
} finally {
  Pop-Location
}
