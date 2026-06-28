# Windows NSIS installer smoke.
# Usage:
#   pwsh scripts/smoke-nsis.ps1
#   pwsh scripts/smoke-nsis.ps1 -InstallerPath ".\desktop\src-tauri\target\x86_64-pc-windows-gnu\release\bundle\nsis\AI Terminal_0.3.3_x64-setup.exe"
param(
  [string]$InstallerPath = '',
  [string]$InstallDir = '',
  [string]$EvidencePath = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent
if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
  $InstallerPath = Join-Path $repoRoot 'desktop\src-tauri\target\x86_64-pc-windows-gnu\release\bundle\nsis\AI Terminal_0.3.3_x64-setup.exe'
}
$InstallerPath = (Resolve-Path -LiteralPath $InstallerPath).Path

$smokeRoot = Join-Path $repoRoot 'artifacts\nsis-install-smoke'
if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  $InstallDir = Join-Path $smokeRoot 'AITerminal'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $smokeRoot 'nsis-smoke-evidence.json'
}

$resolvedSmokeRoot = [System.IO.Path]::GetFullPath($smokeRoot)
$resolvedInstallDir = [System.IO.Path]::GetFullPath($InstallDir)
if (-not $resolvedInstallDir.StartsWith($resolvedSmokeRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "InstallDir must stay under smoke root: $resolvedSmokeRoot"
}

function Get-AiTerminalShellArtifacts {
  $locations = @(
    [Environment]::GetFolderPath('Desktop'),
    [Environment]::GetFolderPath('CommonDesktopDirectory'),
    [Environment]::GetFolderPath('StartMenu'),
    [Environment]::GetFolderPath('CommonStartMenu')
  )

  foreach ($location in $locations) {
    if ($location -and (Test-Path -LiteralPath $location)) {
      Get-ChildItem -Path $location -Recurse -Filter '*AI Terminal*' -ErrorAction SilentlyContinue
    }
  }
}

function Get-AiTerminalUninstallEntries {
  Get-ChildItem -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue |
    Where-Object { $_.GetValue('DisplayName') -like '*AI Terminal*' } |
    ForEach-Object {
      [pscustomobject]@{
        path = $_.PSPath
        displayName = $_.GetValue('DisplayName')
        installLocation = $_.GetValue('InstallLocation')
      }
    }
}

New-Item -ItemType Directory -Force -Path $smokeRoot | Out-Null
if (Test-Path -LiteralPath $resolvedInstallDir) {
  Remove-Item -LiteralPath $resolvedInstallDir -Recurse -Force
}

$installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $InstallerPath).Hash.ToLowerInvariant()
$install = Start-Process -FilePath $InstallerPath -ArgumentList @('/S', "/D=$resolvedInstallDir") -Wait -PassThru
if ($install.ExitCode -ne 0) {
  throw "installer exited with code $($install.ExitCode)"
}

$required = @('ai-terminal.exe', 'ash.exe', 'ai.exe', 'uninstall.exe')
$optional = @('WebView2Loader.dll')
$installedFiles = @()
$missingOptional = @()
foreach ($name in $required) {
  $path = Join-Path $resolvedInstallDir $name
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "installed file missing: $path"
  }
  $installedFiles += $name
}
foreach ($name in $optional) {
  $path = Join-Path $resolvedInstallDir $name
  if (Test-Path -LiteralPath $path -PathType Leaf) {
    $installedFiles += $name
  } else {
    $missingOptional += $name
    Write-Warning "optional installed file not present: $path"
  }
}

# NSIS can return before Windows has released every installer-created file
# handle. Let the freshly installed tree settle before launching the GUI.
Start-Sleep -Milliseconds 1500

$installedSmokeEvidence = Join-Path $smokeRoot 'installed-gui-smoke-evidence.json'
$installedSmokeScreenshot = Join-Path $smokeRoot 'installed-gui-smoke-screenshot.png'
$installedSmokeResizeScreenshot = Join-Path $smokeRoot 'installed-gui-smoke-resize-screenshot.png'
$installedSmokeCtrlCScreenshot = Join-Path $smokeRoot 'installed-gui-smoke-ctrl-c-screenshot.png'
$installedSmokeCtrlDScreenshot = Join-Path $smokeRoot 'installed-gui-smoke-ctrl-d-screenshot.png'
$installedSmokeFrontendEvidence = Join-Path $smokeRoot 'installed-gui-smoke-frontend-evidence.json'
$installedSmokeFrontendScreenshot = Join-Path $smokeRoot 'installed-gui-smoke-frontend-screenshot.png'
$installedSmokeAshIntegrationEvidence = Join-Path $smokeRoot 'installed-gui-smoke-ash-integration-evidence.json'
$installedSmokeAshIntegrationScreenshot = Join-Path $smokeRoot 'installed-gui-smoke-ash-integration-screenshot.png'
$installedSmokeAshIntegrationConfigRoot = Join-Path $smokeRoot 'installed-gui-smoke-config'
$installedSmokeAshIntegrationDataRoot = Join-Path $smokeRoot 'installed-gui-smoke-data'
$installedSmokeTranscript = Join-Path $smokeRoot 'installed-gui-smoke-transcript.txt'
& (Join-Path $PSScriptRoot 'smoke-gui.ps1') `
  -PackageDir $resolvedInstallDir `
  -EvidencePath $installedSmokeEvidence `
  -ScreenshotPath $installedSmokeScreenshot `
  -ResizeScreenshotPath $installedSmokeResizeScreenshot `
  -CtrlCScreenshotPath $installedSmokeCtrlCScreenshot `
  -CtrlDScreenshotPath $installedSmokeCtrlDScreenshot `
  -FrontendEvidencePath $installedSmokeFrontendEvidence `
  -FrontendScreenshotPath $installedSmokeFrontendScreenshot `
  -AshIntegrationEvidencePath $installedSmokeAshIntegrationEvidence `
  -AshIntegrationScreenshotPath $installedSmokeAshIntegrationScreenshot `
  -AshIntegrationConfigRoot $installedSmokeAshIntegrationConfigRoot `
  -AshIntegrationDataRoot $installedSmokeAshIntegrationDataRoot `
  -TranscriptPath $installedSmokeTranscript `
  -SkipChecksums

$uninstaller = Join-Path $resolvedInstallDir 'uninstall.exe'
$uninstall = Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru
if ($uninstall.ExitCode -ne 0) {
  throw "uninstaller exited with code $($uninstall.ExitCode)"
}
Start-Sleep -Milliseconds 500

$remainingFiles = @()
if (Test-Path -LiteralPath $resolvedInstallDir) {
  $remainingFiles = @(Get-ChildItem -Recurse -Force -LiteralPath $resolvedInstallDir -ErrorAction SilentlyContinue)
}
$shortcuts = @(Get-AiTerminalShellArtifacts)
$uninstallEntries = @(Get-AiTerminalUninstallEntries)

if ($remainingFiles.Count -gt 0) {
  throw "installer smoke left files in install dir: $($remainingFiles.FullName -join ', ')"
}
if ($shortcuts.Count -gt 0) {
  throw "installer smoke left shell artifacts: $($shortcuts.FullName -join ', ')"
}
if ($uninstallEntries.Count -gt 0) {
  throw "installer smoke left uninstall entries: $($uninstallEntries.displayName -join ', ')"
}

$evidence = [pscustomobject]@{
  status = 'passed'
  timestamp = (Get-Date).ToString('o')
  installer = $InstallerPath
  installerSha256 = $installerHash
  installDir = $resolvedInstallDir
  installedFiles = $installedFiles
  missingOptionalFiles = $missingOptional
  installedGuiSmokeEvidence = $installedSmokeEvidence
  installedGuiSmokeScreenshot = $installedSmokeScreenshot
  installedGuiSmokeResizeScreenshot = $installedSmokeResizeScreenshot
  installedGuiSmokeCtrlCScreenshot = $installedSmokeCtrlCScreenshot
  installedGuiSmokeCtrlDScreenshot = $installedSmokeCtrlDScreenshot
  installedGuiSmokeFrontendEvidence = $installedSmokeFrontendEvidence
  installedGuiSmokeFrontendScreenshot = $installedSmokeFrontendScreenshot
  installedGuiSmokeAshIntegrationEvidence = $installedSmokeAshIntegrationEvidence
  installedGuiSmokeAshIntegrationScreenshot = $installedSmokeAshIntegrationScreenshot
  installedGuiSmokeAshIntegrationConfigRoot = $installedSmokeAshIntegrationConfigRoot
  installedGuiSmokeAshIntegrationDataRoot = $installedSmokeAshIntegrationDataRoot
  installedGuiSmokeTranscript = $installedSmokeTranscript
  installExitCode = $install.ExitCode
  uninstallExitCode = $uninstall.ExitCode
}
$evidence | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
Write-Output "NSIS_SMOKE_OK $EvidencePath"
