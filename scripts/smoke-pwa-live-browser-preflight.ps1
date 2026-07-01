# RA/PWA live P4b browser/operator evidence preflight.
# Usage:
#   pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-browser-preflight.ps1
#   pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-browser-preflight.ps1 -SkipLiveHarness
param(
  [string]$EvidencePath = '',
  [switch]$SkipLiveHarness
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
$evidenceRoot = Join-Path $repoRoot 'artifacts\ra-pwa-live-browser-preflight'
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $evidenceRoot 'ra-pwa-live-browser-preflight.json'
}
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null

function Invoke-PreflightStep {
  param(
    [string]$Name,
    [scriptblock]$Script,
    [switch]$AllowBlocked
  )

  $started = Get-Date
  $global:LASTEXITCODE = 0
  try {
    $output = @(& $Script 2>&1)
    $exitCode = if ($null -ne $global:LASTEXITCODE) { [int]$global:LASTEXITCODE } else { 0 }
    $status = if ($exitCode -eq 0) {
      'passed'
    } elseif ($AllowBlocked -and $exitCode -eq 88) {
      'blocked'
    } else {
      'failed'
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

function Test-RequiredFiles {
  $required = @(
    'pwa\index.html',
    'pwa\app.mjs',
    'pwa\app.test.mjs',
    'pwa\styles.css',
    'scripts\smoke-pwa-live-approval.ps1'
  )
  $missing = @()
  foreach ($relative in $required) {
    $path = Join-Path $repoRoot $relative
    if (-not (Test-Path -LiteralPath $path)) {
      $missing += $relative
    }
  }
  if ($missing.Count -gt 0) {
    throw "missing required files: $($missing -join ', ')"
  }
  "REQUIRED_FILES_OK"
}

function Test-PwaBrowserSurface {
  $index = Get-Content -LiteralPath (Join-Path $repoRoot 'pwa\index.html') -Raw
  $app = Get-Content -LiteralPath (Join-Path $repoRoot 'pwa\app.mjs') -Raw
  $required = @(
    'id="live-endpoint"',
    'id="live-connect-button"',
    'id="live-disconnect-button"',
    'id="live-state"',
    'id="live-last-event"',
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
    throw "missing PWA browser evidence surface: $($missing -join ', ')"
  }
  "PWA_BROWSER_SURFACE_OK"
}

function Test-NodeAvailable {
  $node = Get-Command 'node' -ErrorAction SilentlyContinue
  if (-not $node) {
    Write-Output 'node not found'
    $global:LASTEXITCODE = 88
    return
  }
  & node --version
}

function Test-WslRustAvailable {
  $wsl = Get-Command 'wsl.exe' -ErrorAction SilentlyContinue
  if (-not $wsl) {
    Write-Output 'wsl.exe not found'
    $global:LASTEXITCODE = 88
    return
  }
  $env:MSYS_NO_PATHCONV = '1'
  & wsl.exe -- bash -lc 'source ~/.cargo/env >/dev/null 2>&1; cargo --version && rustc --version'
}

function Test-PlaywrightAvailable {
  $node = Get-Command 'node' -ErrorAction SilentlyContinue
  if (-not $node) {
    Write-Output 'node not found'
    $global:LASTEXITCODE = 88
    return
  }
  $script = @"
import('playwright')
  .then((pw) => {
    const engines = ['chromium', 'firefox', 'webkit'].filter((name) => Boolean(pw[name]));
    console.log('PLAYWRIGHT_OK ' + engines.join(','));
  })
  .catch((err) => {
    console.log('playwright import failed: ' + err.message);
    process.exit(88);
  });
"@
  & node -e $script
}

function Test-BrowserBinaryAvailable {
  $pathCandidates = @(
    'msedge',
    'msedge.exe',
    'chrome',
    'chrome.exe',
    'chromium',
    'chromium-browser'
  )
  foreach ($candidate in $pathCandidates) {
    $command = Get-Command $candidate -ErrorAction SilentlyContinue
    if ($command) {
      "BROWSER_BINARY_OK $($command.Source)"
      return
    }
  }

  $installCandidates = @(
    (Join-Path $env:ProgramFiles 'Google\Chrome\Application\chrome.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Google\Chrome\Application\chrome.exe'),
    (Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'),
    (Join-Path $env:LOCALAPPDATA 'Google\Chrome\Application\chrome.exe'),
    (Join-Path $env:LOCALAPPDATA 'Microsoft\Edge\Application\msedge.exe')
  )
  foreach ($candidate in $installCandidates) {
    if ([string]::IsNullOrWhiteSpace($candidate)) {
      continue
    }
    if (Test-Path -LiteralPath $candidate) {
      "BROWSER_BINARY_OK $candidate"
      return
    }
  }

  Write-Output 'no Edge/Chrome/Chromium command found on PATH or common install paths'
  $global:LASTEXITCODE = 88
}

Push-Location $repoRoot
try {
  $steps = @()
  $steps += Invoke-PreflightStep -Name 'required-files' -Script {
    Test-RequiredFiles
  }
  $steps += Invoke-PreflightStep -Name 'node-available' -AllowBlocked -Script {
    Test-NodeAvailable
  }
  $steps += Invoke-PreflightStep -Name 'pwa-node-helper-tests' -Script {
    & node 'pwa/app.test.mjs'
  }
  $steps += Invoke-PreflightStep -Name 'pwa-browser-surface' -Script {
    Test-PwaBrowserSurface
  }
  $steps += Invoke-PreflightStep -Name 'wsl-rust-toolchain' -AllowBlocked -Script {
    Test-WslRustAvailable
  }

  if ($SkipLiveHarness) {
    $steps += [pscustomobject]@{
      name = 'p4a-live-harness'
      status = 'skipped'
      exitCode = 0
      startedAt = (Get-Date).ToString('o')
      finishedAt = (Get-Date).ToString('o')
      outputTail = @('SkipLiveHarness requested')
    }
  } else {
    $steps += Invoke-PreflightStep -Name 'p4a-live-harness' -Script {
      & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot 'scripts\smoke-pwa-live-approval.ps1')
    }
  }

  $steps += Invoke-PreflightStep -Name 'playwright-automation' -AllowBlocked -Script {
    Test-PlaywrightAvailable
  }
  $steps += Invoke-PreflightStep -Name 'browser-binary-path' -AllowBlocked -Script {
    Test-BrowserBinaryAvailable
  }

  $failed = @($steps | Where-Object { $_.status -eq 'failed' })
  $blocked = @($steps | Where-Object { $_.status -eq 'blocked' })
  $overall = if ($failed.Count -gt 0) {
    'failed'
  } elseif ($blocked.Count -gt 0) {
    'blocked'
  } else {
    'ready'
  }

  $evidence = [pscustomobject]@{
    status = $overall
    timestamp = (Get-Date).ToString('o')
    repoRoot = $repoRoot
    evidencePath = $EvidencePath
    objective = 'Prepare RA/PWA P4b browser/operator live approval evidence'
    steps = $steps
    readyCriteria = @(
      'P4a live harness passes',
      'PWA browser live controls and pending approval surface exist',
      'WSL Rust remote tests are runnable',
      'Browser automation is available for screenshot/transcript capture',
      'A browser binary is available for operator evidence'
    )
    nextActions = @(
      'Start isolated XDG_CONFIG_HOME/XDG_DATA_HOME for P4b',
      'Pair disposable PWA identity with ai remote pair',
      'Start ai remote daemon --device-id <id>',
      'Connect PWA to printed live endpoint',
      'Capture approve and reject High command evidence'
    )
  }
  $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

  if ($overall -eq 'ready') {
    Write-Output "RA_PWA_LIVE_BROWSER_PREFLIGHT_READY $EvidencePath"
    exit 0
  }
  if ($overall -eq 'blocked') {
    Write-Output "RA_PWA_LIVE_BROWSER_PREFLIGHT_BLOCKED $EvidencePath"
    exit 2
  }
  Write-Output "RA_PWA_LIVE_BROWSER_PREFLIGHT_FAILED $EvidencePath"
  exit 1
} finally {
  Pop-Location
}
