# Deterministic smoke for scripts/show-release-followup-status.ps1.
#
# This script uses synthetic evidence only. It does not read GitHub secrets,
# build MSI artifacts, or run fdroidserver.
param(
  [string]$SmokeRoot = '',
  [string]$EvidencePath = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
if ([string]::IsNullOrWhiteSpace($SmokeRoot)) {
  $SmokeRoot = Join-Path $repoRoot 'artifacts\release-followup-status-smoke'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $SmokeRoot 'release-followup-status-smoke-evidence.json'
}
New-Item -ItemType Directory -Force -Path $SmokeRoot | Out-Null

function New-CloseoutItem {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [string]$Status,
    [string[]]$Missing = @()
  )

  [pscustomobject]@{
    name = $Name
    status = $Status
    evidencePath = $null
    required = @('synthetic required evidence')
    missing = @($Missing)
    note = "Synthetic $Name $Status"
  }
}

function New-ReleaseFollowupEvidence {
  param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('ready', 'blocked')]
    [string]$Status
  )

  $items = if ($Status -eq 'ready') {
    @(
      New-CloseoutItem -Name 'msi' -Status 'ready'
      New-CloseoutItem -Name 'androidSigningSecrets' -Status 'ready'
      New-CloseoutItem -Name 'fdroidBuild' -Status 'ready'
    )
  } else {
    @(
      New-CloseoutItem -Name 'msi' -Status 'blocked' -Missing @('RunMsiBuild evidence')
      New-CloseoutItem -Name 'androidSigningSecrets' -Status 'blocked' -Missing @('repository secret names')
      New-CloseoutItem -Name 'fdroidBuild' -Status 'blocked' -Missing @('evidencePath')
    )
  }

  $blockedItems = @($items | Where-Object { $_.status -ne 'ready' } | ForEach-Object { $_.name })
  $readyItems = @($items | Where-Object { $_.status -eq 'ready' } | ForEach-Object { $_.name })
  $canCloseDocs = ($Status -eq 'ready' -and $blockedItems.Count -eq 0)

  [pscustomobject]@{
    status = $Status
    timestamp = (Get-Date).ToString('o')
    repoRoot = $repoRoot
    closeout = [pscustomobject]@{
      status = $Status
      ready = $canCloseDocs
      canCloseDocs = $canCloseDocs
      requiredEvidence = @('msi', 'androidSigningSecrets', 'fdroidBuild')
      readyItems = $readyItems
      blockedItems = $blockedItems
      items = $items
      releaseTagAction = 'unchanged'
      assetAction = 'unchanged'
      note = if ($canCloseDocs) {
        'Synthetic ready closeout'
      } else {
        'Synthetic blocked closeout'
      }
    }
    blockers = if ($Status -eq 'ready') {
      @()
    } else {
      @(
        'Synthetic MSI blocker',
        'Synthetic Android signing blocker',
        'Synthetic F-Droid blocker'
      )
    }
    nextActions = if ($Status -eq 'ready') {
      @('Close release follow-up docs')
    } else {
      @(
        'Produce MSI evidence',
        'Register Android signing secret names',
        'Capture F-Droid build evidence'
      )
    }
  }
}

function Write-SyntheticEvidence {
  param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('ready', 'blocked')]
    [string]$Status,
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  New-ReleaseFollowupEvidence -Status $Status |
    ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath $Path -Encoding utf8
}

function Invoke-Status {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [string[]]$ExtraArgs = @()
  )

  $args = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', (Join-Path $repoRoot 'scripts\show-release-followup-status.ps1'),
    '-EvidencePath', $Path
  ) + $ExtraArgs

  $output = @(& pwsh @args 2>&1)
  [pscustomobject]@{
    exitCode = $LASTEXITCODE
    output = @($output | ForEach-Object { "$_" })
  }
}

function Assert-True {
  param(
    [bool]$Condition,
    [string]$Message
  )

  if (-not $Condition) {
    throw $Message
  }
}

$blockedPath = Join-Path $SmokeRoot 'blocked-evidence.json'
$readyPath = Join-Path $SmokeRoot 'ready-evidence.json'
Write-SyntheticEvidence -Status 'blocked' -Path $blockedPath
Write-SyntheticEvidence -Status 'ready' -Path $readyPath

$checks = @()

$blockedText = Invoke-Status -Path $blockedPath
Assert-True ($blockedText.exitCode -eq 0) 'blocked text status should exit 0 without -FailOnBlocked'
Assert-True (($blockedText.output -join "`n") -match 'RELEASE_FOLLOWUP_STATUS blocked') 'blocked text output missing status marker'
Assert-True (($blockedText.output -join "`n") -match 'Synthetic MSI blocker') 'blocked text output missing blocker'
$checks += [pscustomobject]@{ name = 'blocked-text'; status = 'passed' }

$blockedJson = Invoke-Status -Path $blockedPath -ExtraArgs @('-Json')
Assert-True ($blockedJson.exitCode -eq 0) 'blocked JSON status should exit 0 without -FailOnBlocked'
$blockedSummary = ($blockedJson.output -join "`n") | ConvertFrom-Json
Assert-True ($blockedSummary.status -eq 'blocked') 'blocked JSON status mismatch'
Assert-True (-not [bool]$blockedSummary.canCloseDocs) 'blocked JSON should not allow doc closeout'
Assert-True (@($blockedSummary.blockedItems).Count -eq 3) 'blocked JSON should report three blocked items'
$checks += [pscustomobject]@{ name = 'blocked-json'; status = 'passed' }

$blockedGate = Invoke-Status -Path $blockedPath -ExtraArgs @('-FailOnBlocked')
Assert-True ($blockedGate.exitCode -eq 2) 'blocked gate should exit 2 with -FailOnBlocked'
$checks += [pscustomobject]@{ name = 'blocked-fail-on-blocked'; status = 'passed' }

$readyText = Invoke-Status -Path $readyPath -ExtraArgs @('-FailOnBlocked')
Assert-True ($readyText.exitCode -eq 0) 'ready gate should exit 0 with -FailOnBlocked'
Assert-True (($readyText.output -join "`n") -match 'RELEASE_FOLLOWUP_STATUS ready') 'ready text output missing status marker'
Assert-True (($readyText.output -join "`n") -match 'Can close docs: True') 'ready text output should allow doc closeout'
$checks += [pscustomobject]@{ name = 'ready-text-fail-on-blocked'; status = 'passed' }

$readyJson = Invoke-Status -Path $readyPath -ExtraArgs @('-Json')
Assert-True ($readyJson.exitCode -eq 0) 'ready JSON status should exit 0'
$readySummary = ($readyJson.output -join "`n") | ConvertFrom-Json
Assert-True ($readySummary.status -eq 'ready') 'ready JSON status mismatch'
Assert-True ([bool]$readySummary.canCloseDocs) 'ready JSON should allow doc closeout'
Assert-True (@($readySummary.blockedItems).Count -eq 0) 'ready JSON should report zero blocked items'
Assert-True (@($readySummary.readyItems).Count -eq 3) 'ready JSON should report three ready items'
$checks += [pscustomobject]@{ name = 'ready-json'; status = 'passed' }

$evidence = [pscustomobject]@{
  status = 'passed'
  timestamp = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  smokeRoot = [System.IO.Path]::GetFullPath($SmokeRoot)
  blockedEvidencePath = [System.IO.Path]::GetFullPath($blockedPath)
  readyEvidencePath = [System.IO.Path]::GetFullPath($readyPath)
  checks = $checks
}
$evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

Write-Output "RELEASE_FOLLOWUP_STATUS_SMOKE_OK $EvidencePath"
