# Export an operator-facing packet for the external v0.3.3 release follow-up.
#
# The packet is intentionally secret-free. It records only blocker names,
# required secret names, evidence paths, commands, and docs references.
param(
  [string]$EvidencePath = '',
  [string]$PacketRoot = '',
  [switch]$Refresh,
  [switch]$RunMsiBuild,
  [switch]$RunAndroidLocalSmokes,
  [string]$FdroidBuildEvidencePath = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $repoRoot 'artifacts\release-followup-preflight\release-followup-preflight-evidence.json'
}
if ([string]::IsNullOrWhiteSpace($PacketRoot)) {
  $PacketRoot = Join-Path $repoRoot 'artifacts\release-followup-evidence-packet'
}

New-Item -ItemType Directory -Force -Path $PacketRoot | Out-Null
$packetJsonPath = Join-Path $PacketRoot 'release-followup-evidence-packet.json'
$packetMarkdownPath = Join-Path $PacketRoot 'release-followup-evidence-packet.md'

function Invoke-ReleaseFollowupCheck {
  $args = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', (Join-Path $repoRoot 'scripts\check-release-followup.ps1'),
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

  & pwsh @args | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "release follow-up check failed with exit code $LASTEXITCODE"
  }
}

function Get-Array {
  param([object]$Value)
  @($Value | Where-Object { $null -ne $_ })
}

if ($Refresh -or -not (Test-Path -LiteralPath $EvidencePath -PathType Leaf)) {
  Invoke-ReleaseFollowupCheck
}

if (-not (Test-Path -LiteralPath $EvidencePath -PathType Leaf)) {
  throw "release follow-up evidence not found: $EvidencePath"
}

$statusJson = & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot 'scripts\show-release-followup-status.ps1') `
  -EvidencePath $EvidencePath `
  -Json
if ($LASTEXITCODE -ne 0) {
  throw "release follow-up status failed with exit code $LASTEXITCODE"
}

$status = $statusJson | ConvertFrom-Json
$evidence = Get-Content -Raw -LiteralPath $EvidencePath | ConvertFrom-Json
if (-not $evidence.closeout) {
  throw "release follow-up evidence does not include closeout"
}

$externalCommands = [ordered]@{
  msi = @(
    'pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 -RunMsiBuild',
    'Get-Content artifacts\release-followup-preflight\msi-preflight-evidence.json -Raw | ConvertFrom-Json'
  )
  androidSigningSecrets = @(
    'gh secret set AI_TERMINAL_ANDROID_KEYSTORE_BASE64',
    'gh secret set AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD',
    'gh secret set AI_TERMINAL_ANDROID_KEY_ALIAS',
    'gh secret set AI_TERMINAL_ANDROID_KEY_PASSWORD',
    'gh secret list --json name,updatedAt',
    'npm run smoke:release-followup-preflight'
  )
  fdroidBuild = @(
    'pwsh -NoProfile -ExecutionPolicy Bypass -File .\android\smoke-fdroid-release-activation.ps1 -Commit <40-char-release-commit>',
    'fdroid build dev.aiterminal.android:303',
    'pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 -FdroidBuildEvidencePath <path-to-fdroid-build-evidence.json>'
  )
  finalCloseout = @(
    'npm run check:release-followup',
    'npm run status:release-followup'
  )
}

$packet = [pscustomobject]@{
  status = $status.status
  canCloseDocs = [bool]$status.canCloseDocs
  generatedAt = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  evidencePath = [System.IO.Path]::GetFullPath($EvidencePath)
  packetJsonPath = [System.IO.Path]::GetFullPath($packetJsonPath)
  packetMarkdownPath = [System.IO.Path]::GetFullPath($packetMarkdownPath)
  readyItems = Get-Array $status.readyItems
  blockedItems = Get-Array $status.blockedItems
  blockers = Get-Array $status.blockers
  nextActions = Get-Array $status.nextActions
  closeout = $evidence.closeout
  msi = [pscustomobject]@{
    status = $evidence.msi.status
    evidencePath = $evidence.msi.evidencePath
    missing = Get-Array $evidence.msi.missing
    checks = $evidence.msi.checks
    build = $evidence.msi.build
  }
  androidSigningSecrets = [pscustomobject]@{
    status = $evidence.androidSigningSecrets.status
    required = Get-Array $evidence.androidSigningSecrets.required
    workflowStatus = $evidence.androidSigningSecrets.workflow.status
    workflowPath = $evidence.androidSigningSecrets.workflow.path
    workflowMissing = Get-Array $evidence.androidSigningSecrets.workflow.missing
    present = Get-Array $evidence.androidSigningSecrets.present
    missing = Get-Array $evidence.androidSigningSecrets.missing
    note = 'Secret values are never read, printed, or persisted by this packet'
  }
  fdroidBuild = [pscustomobject]@{
    status = $evidence.fdroidBuild.status
    evidencePath = $evidence.fdroidBuild.evidencePath
    expected = $evidence.fdroidExpectations
    missing = Get-Array $evidence.fdroidBuild.missing
    checks = $evidence.fdroidBuild.checks
  }
  externalCommands = $externalCommands
  docs = @(
    'docs/releases/release-followup-runbook.md',
    'docs/TROUBLESHOOTING.md',
    'docs/superpowers/plans/2026-07-01-remaining-work-priority.md',
    'docs/HANDOFF.md'
  )
  safetyRules = @(
    'Do not commit keystores, password files, decoded secrets, APK signing material, or artifacts output',
    'Record GitHub Android signing secret names only; never record secret values',
    'Do not mark release follow-up docs closed until closeout.canCloseDocs is true and closeout.blockedItems is empty',
    'Keep release tag and existing assets unchanged unless there is a separate release decision'
  )
}

$packet | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $packetJsonPath -Encoding utf8

$lines = @()
$lines += '# Release Follow-up Evidence Packet'
$lines += ''
$lines += "Generated: $($packet.generatedAt)"
$lines += "Status: $($packet.status)"
$lines += "Can close docs: $($packet.canCloseDocs)"
$lines += "Evidence: $($packet.evidencePath)"
$lines += ''
$lines += '## Blocked Items'
$blockedItems = Get-Array $packet.blockedItems
if ($blockedItems.Count -eq 0) {
  $lines += '- None'
} else {
  foreach ($item in $blockedItems) {
    $lines += "- $item"
  }
}
$lines += ''
$lines += '## Next Actions'
$nextActions = Get-Array $packet.nextActions
if ($nextActions.Count -eq 0) {
  $lines += '- None'
} else {
  for ($i = 0; $i -lt $nextActions.Count; $i += 1) {
    $lines += "$($i + 1). $($nextActions[$i])"
  }
}
$lines += ''
$lines += '## External Commands'
foreach ($key in $externalCommands.Keys) {
  $lines += ''
  $lines += "### $key"
  foreach ($command in @($externalCommands[$key])) {
    $lines += ''
    $lines += '```powershell'
    $lines += $command
    $lines += '```'
  }
}
$lines += ''
$lines += '## Safety Rules'
foreach ($rule in @($packet.safetyRules)) {
  $lines += "- $rule"
}
$lines += ''
$lines += '## Docs'
foreach ($doc in @($packet.docs)) {
  $lines += "- $doc"
}

$lines | Set-Content -LiteralPath $packetMarkdownPath -Encoding utf8
Write-Output "RELEASE_FOLLOWUP_EVIDENCE_PACKET_OK $packetJsonPath $packetMarkdownPath"
