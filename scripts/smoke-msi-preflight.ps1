# Windows MSI packaging preflight.
# Usage:
#   pwsh scripts/smoke-msi-preflight.ps1
#   pwsh scripts/smoke-msi-preflight.ps1 -RunBuild
param(
  [string]$EvidencePath = '',
  [switch]$RunBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
$desktopRoot = Join-Path $repoRoot 'desktop'
$evidenceRoot = Join-Path $repoRoot 'artifacts\msi-preflight'
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $evidenceRoot 'msi-preflight-evidence.json'
}
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null

function Get-ToolInfo {
  param([string]$Name, [string[]]$VersionArgs = @('--version'))
  $cmd = Get-Command $Name -ErrorAction SilentlyContinue
  $version = $null
  if ($cmd) {
    try {
      $version = (& $Name @VersionArgs 2>$null | Select-Object -First 1)
    } catch {
      $version = $null
    }
  }
  [pscustomobject]@{
    command = $Name
    path = if ($cmd) { $cmd.Source } else { $null }
    version = $version
    present = [bool]$cmd
  }
}

$tools = @(
  (Get-ToolInfo 'cargo'),
  (Get-ToolInfo 'rustc'),
  (Get-ToolInfo 'cl'),
  (Get-ToolInfo 'link'),
  (Get-ToolInfo 'rc'),
  (Get-ToolInfo 'wix'),
  (Get-ToolInfo 'heat'),
  (Get-ToolInfo 'candle'),
  (Get-ToolInfo 'light'),
  (Get-ToolInfo 'node'),
  (Get-ToolInfo 'npm')
)

$required = @('cargo', 'rustc', 'cl', 'link', 'rc')
$presentCommands = @($tools | Where-Object { $_.present } | ForEach-Object { $_.command })
$wixAny = @('wix', 'heat', 'candle', 'light') | Where-Object {
  $presentCommands -contains $_
}
$missing = @()
foreach ($name in $required) {
  if (-not (($tools | Where-Object { $_.command -eq $name -and $_.present }).Count -gt 0)) {
    $missing += $name
  }
}
if ($wixAny.Count -eq 0) {
  $missing += 'wix-or-wix-toolset'
}

$build = $null
if ($RunBuild -and $missing.Count -eq 0) {
  Push-Location $desktopRoot
  try {
    $output = & npm run tauri -- build --bundles msi --ci 2>&1
    $exit = $LASTEXITCODE
    $msi = Get-ChildItem -Path (Join-Path $desktopRoot 'src-tauri\target') -Recurse -Filter '*.msi' -File -ErrorAction SilentlyContinue |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1
    $build = [pscustomobject]@{
      attempted = $true
      exitCode = $exit
      outputTail = @($output | Select-Object -Last 80)
      msiPath = if ($msi) { $msi.FullName } else { $null }
      msiSha256 = if ($msi) { (Get-FileHash -Algorithm SHA256 -LiteralPath $msi.FullName).Hash.ToLowerInvariant() } else { $null }
    }
  } finally {
    Pop-Location
  }
} else {
  $build = [pscustomobject]@{
    attempted = $false
    reason = if ($missing.Count -gt 0) { "missing required MSI toolchain: $($missing -join ', ')" } else { 'RunBuild not requested' }
  }
}

$status = if ($missing.Count -eq 0) { 'ready' } else { 'blocked' }
$evidence = [pscustomobject]@{
  status = $status
  timestamp = (Get-Date).ToString('o')
  repoRoot = $repoRoot
  desktopRoot = $desktopRoot
  required = $required
  wixRequirement = 'wix CLI or WiX Toolset heat/candle/light'
  missing = $missing
  tools = $tools
  build = $build
}
$evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8

if ($status -eq 'blocked') {
  Write-Output "MSI_PREFLIGHT_BLOCKED $EvidencePath"
} else {
  Write-Output "MSI_PREFLIGHT_READY $EvidencePath"
}
