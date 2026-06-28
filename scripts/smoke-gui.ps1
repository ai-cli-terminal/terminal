# Windows GUI portable package smoke.
# Usage:
#   pwsh scripts/smoke-gui.ps1
#   pwsh scripts/smoke-gui.ps1 -PackageDir .\desktop\src-tauri\target\x86_64-pc-windows-gnu\release\portable\ai-terminal-windows-x86_64-pc-windows-gnu
#   pwsh scripts/smoke-gui.ps1 -PackageDir .\artifacts\nsis-install-smoke\AITerminal -SkipChecksums
#   pwsh scripts/smoke-gui.ps1 -Interactive
param(
  [string]$PackageDir = '',
  [int]$StartupTimeoutSeconds = 20,
  [string]$EvidencePath = '',
  [string]$ScreenshotPath = '',
  [string]$ResizeScreenshotPath = '',
  [string]$CtrlCScreenshotPath = '',
  [string]$CtrlDScreenshotPath = '',
  [string]$FrontendEvidencePath = '',
  [string]$FrontendScreenshotPath = '',
  [string]$AshIntegrationEvidencePath = '',
  [string]$AshIntegrationScreenshotPath = '',
  [string]$AshIntegrationConfigRoot = '',
  [string]$AshIntegrationDataRoot = '',
  [string]$TranscriptPath = '',
  [string]$SmokeCommand = 'print AI_TERMINAL_GUI_SMOKE_OK',
  [string]$ExpectedOutput = 'AI_TERMINAL_GUI_SMOKE_OK',
  [string]$UnexpectedOutput = 'error:',
  [string]$CtrlCInput = 'AI_TERMINAL_GUI_SMOKE_CTRL_C_PENDING',
  [string]$CtrlCRecoveryCommand = 'print AI_TERMINAL_GUI_SMOKE_CTRL_C_OK',
  [string]$CtrlCExpectedOutput = 'AI_TERMINAL_GUI_SMOKE_CTRL_C_OK',
  [int]$CtrlCDelayMilliseconds = 2500,
  [int]$CtrlDDelayMilliseconds = 16000,
  [string]$FrontendSelectionText = 'AI_TERMINAL_GUI_SMOKE_SELECTION_TEXT',
  [string]$FrontendPasteText = "print AI_TERMINAL_GUI_SMOKE_PASTE_OK`r",
  [string]$FrontendExpectedOutput = 'AI_TERMINAL_GUI_SMOKE_PASTE_OK',
  [int]$FrontendDelayMilliseconds = 4200,
  [int]$FrontendScrollbackLines = 120,
  [string]$AshIntegrationCommands = "ai AI_TERMINAL_GUI_SMOKE_AI_ROUTE`nrm -rf /`ncmd.exe /c echo AI_TERMINAL_GUI_SMOKE_EXTERNAL_OK",
  [string]$AshIntegrationAiMarker = 'AI_TERMINAL_GUI_SMOKE_AI_ROUTE',
  [string]$AshIntegrationExternalMarker = 'AI_TERMINAL_GUI_SMOKE_EXTERNAL_OK',
  [int]$AshIntegrationDelayMilliseconds = 7000,
  [int]$AshIntegrationCommandIntervalMilliseconds = 1500,
  [switch]$SkipChecksums,
  [switch]$SkipCommandSmoke,
  [switch]$SkipResizeSmoke,
  [switch]$SkipCtrlCSmoke,
  [switch]$SkipCtrlDSmoke,
  [switch]$SkipFrontendSmoke,
  [switch]$SkipAshIntegrationSmoke,
  [switch]$Interactive
)

$ErrorActionPreference = 'Stop'

function Resolve-DefaultPackageDir {
  param(
    [string]$RepoRoot,
    [string]$ScriptRoot
  )
  if (Test-Path -LiteralPath (Join-Path $ScriptRoot 'ai-terminal.exe') -PathType Leaf) {
    return $ScriptRoot
  }
  return Join-Path $RepoRoot 'desktop\src-tauri\target\x86_64-pc-windows-gnu\release\portable\ai-terminal-windows-x86_64-pc-windows-gnu'
}

function Assert-File {
  param([string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "required file not found: $Path"
  }
}

function Test-Sha256Manifest {
  param([string]$PackageDir)

  $manifest = Join-Path $PackageDir 'SHA256SUMS.txt'
  Assert-File $manifest

  $results = @()
  foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -notmatch '^([0-9a-fA-F]{64})\s+(.+)$') {
      throw "invalid checksum line: $line"
    }

    $expected = $Matches[1].ToLowerInvariant()
    $name = $Matches[2].Trim()
    $path = Join-Path $PackageDir $name
    Assert-File $path

    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
      throw "checksum mismatch for ${name}: expected $expected actual $actual"
    }

    $results += [pscustomobject]@{
      file = $name
      sha256 = $actual
    }
  }

  return $results
}

function Get-DescendantProcessInfo {
  param([int]$RootProcessId)

  $all = Get-CimInstance Win32_Process
  $known = @{}
  $known[$RootProcessId] = $true
  $changed = $true

  while ($changed) {
    $changed = $false
    foreach ($process in $all) {
      if ($known.ContainsKey([int]$process.ParentProcessId) -and -not $known.ContainsKey([int]$process.ProcessId)) {
        $known[[int]$process.ProcessId] = $true
        $changed = $true
      }
    }
  }

  return $all |
    Where-Object { $_.ProcessId -ne $RootProcessId -and $known.ContainsKey([int]$_.ProcessId) } |
    Sort-Object ProcessId
}

function Wait-ForCondition {
  param(
    [scriptblock]$Condition,
    [int]$TimeoutSeconds,
    [string]$FailureMessage
  )

  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    $value = & $Condition
    if ($value) { return $value }
    Start-Sleep -Milliseconds 250
  }

  throw $FailureMessage
}

function Initialize-ScreenshotCapture {
  if ('AiTerminalSmoke.NativeMethods' -as [type]) {
    return
  }

  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace AiTerminalSmoke {
  public static class NativeMethods {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
      public int Left;
      public int Top;
      public int Right;
      public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(
      IntPtr hWnd,
      IntPtr hWndInsertAfter,
      int X,
      int Y,
      int cx,
      int cy,
      uint uFlags
    );
  }
}
'@
}

function Save-WindowScreenshot {
  param(
    [IntPtr]$WindowHandle,
    [string]$Path
  )

  Initialize-ScreenshotCapture

  $rect = New-Object AiTerminalSmoke.NativeMethods+RECT
  $ok = [AiTerminalSmoke.NativeMethods]::GetWindowRect($WindowHandle, [ref]$rect)
  if (-not $ok) {
    throw "failed to read ai-terminal.exe window rectangle"
  }

  $width = $rect.Right - $rect.Left
  $height = $rect.Bottom - $rect.Top
  if ($width -le 0 -or $height -le 0) {
    throw "invalid ai-terminal.exe window rectangle: ${width}x${height}"
  }

  Add-Type -AssemblyName System.Drawing
  $bitmap = New-Object System.Drawing.Bitmap $width, $height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $hdc = $graphics.GetHdc()
    try {
      $printed = [AiTerminalSmoke.NativeMethods]::PrintWindow($WindowHandle, $hdc, 0x00000002)
    } finally {
      $graphics.ReleaseHdc($hdc)
    }
    if (-not $printed) {
      $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
    }
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

function Get-WindowBounds {
  param([IntPtr]$WindowHandle)

  Initialize-ScreenshotCapture
  $rect = New-Object AiTerminalSmoke.NativeMethods+RECT
  $ok = [AiTerminalSmoke.NativeMethods]::GetWindowRect($WindowHandle, [ref]$rect)
  if (-not $ok) {
    return $null
  }

  return [pscustomobject]@{
    left = $rect.Left
    top = $rect.Top
    right = $rect.Right
    bottom = $rect.Bottom
    width = $rect.Right - $rect.Left
    height = $rect.Bottom - $rect.Top
  }
}

function Request-WindowClose {
  param(
    [System.Diagnostics.Process]$Process
  )

  $closed = $Process.CloseMainWindow()
  if (-not $closed -and $Process.MainWindowHandle -ne 0) {
    Initialize-ScreenshotCapture
    [AiTerminalSmoke.NativeMethods]::PostMessage(
      $Process.MainWindowHandle,
      0x0010,
      [IntPtr]::Zero,
      [IntPtr]::Zero
    ) | Out-Null
  }
}

function Invoke-ResizeSmoke {
  param(
    [IntPtr]$WindowHandle,
    [string]$Path,
    [int]$TimeoutSeconds
  )

  $before = Get-WindowBounds -WindowHandle $WindowHandle
  if (-not $before) {
    throw "failed to read ai-terminal.exe window bounds before resize"
  }

  $targetWidth = if ($before.width -gt 820) { $before.width - 180 } else { $before.width + 160 }
  $targetHeight = if ($before.height -gt 560) { $before.height - 120 } else { $before.height + 100 }
  $targetWidth = [Math]::Max(640, [Math]::Min(1200, $targetWidth))
  $targetHeight = [Math]::Max(420, [Math]::Min(900, $targetHeight))

  Initialize-ScreenshotCapture
  $ok = [AiTerminalSmoke.NativeMethods]::SetWindowPos(
    $WindowHandle,
    [IntPtr]::Zero,
    $before.left,
    $before.top,
    $targetWidth,
    $targetHeight,
    0x0044
  )
  if (-not $ok) {
    throw "failed to resize ai-terminal.exe window"
  }

  $after = Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "ai-terminal.exe window did not reach resized bounds within ${TimeoutSeconds}s" `
    -Condition {
      $bounds = Get-WindowBounds -WindowHandle $WindowHandle
      if (-not $bounds) { return $false }
      if ([Math]::Abs($bounds.width - $targetWidth) -le 30 -and [Math]::Abs($bounds.height - $targetHeight) -le 30) {
        return $bounds
      }
      return $false
    }

  Start-Sleep -Milliseconds 750
  Save-WindowScreenshot -WindowHandle $WindowHandle -Path $Path

  return [pscustomobject]@{
    before = $before
    target = [pscustomobject]@{
      width = $targetWidth
      height = $targetHeight
    }
    after = $after
    screenshot = $Path
  }
}

function Invoke-CtrlCSmoke {
  param(
    [System.Diagnostics.Process]$Process,
    [int]$AshProcessId,
    [string]$Path,
    [string]$TranscriptPath,
    [string]$PendingInput,
    [string]$ExpectedOutput,
    [int]$DelayMilliseconds,
    [int]$TimeoutSeconds
  )

  Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "Ctrl-C recovery output did not appear within ${TimeoutSeconds}s: $ExpectedOutput" `
    -Condition {
      if (-not (Test-Path -LiteralPath $TranscriptPath -PathType Leaf)) { return $false }
      $content = Get-Content -LiteralPath $TranscriptPath -Raw -ErrorAction SilentlyContinue
      if ($content -and $content.Contains($ExpectedOutput)) { return $true }
      return $false
    } | Out-Null

  $remaining = @(Get-Process -Name 'ash' -ErrorAction SilentlyContinue | Where-Object { $_.Id -eq $AshProcessId })
  if ($remaining.Count -eq 0) {
    throw "ash.exe child exited during Ctrl-C smoke: pid $AshProcessId"
  }

  $Process.Refresh()
  if ($Process.MainWindowHandle -ne 0) {
    Start-Sleep -Milliseconds 750
    Save-WindowScreenshot -WindowHandle $Process.MainWindowHandle -Path $Path
  }

  return [pscustomobject]@{
    delayMilliseconds = $DelayMilliseconds
    ashProcessId = $AshProcessId
    input = $PendingInput
    expectedOutput = $ExpectedOutput
    recovered = $true
    ashStillRunning = $true
    screenshot = if ($Process.MainWindowHandle -ne 0) { $Path } else { $null }
  }
}

function Invoke-CtrlDSmoke {
  param(
    [System.Diagnostics.Process]$Process,
    [int]$AshProcessId,
    [string]$Path,
    [int]$DelayMilliseconds,
    [int]$TimeoutSeconds
  )

  Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "ash.exe child did not exit after Ctrl-D within ${TimeoutSeconds}s" `
    -Condition {
      $remaining = @(Get-Process -Name 'ash' -ErrorAction SilentlyContinue | Where-Object { $_.Id -eq $AshProcessId })
      if ($remaining.Count -eq 0) { return $true }
      return $false
    } | Out-Null

  $Process.Refresh()
  if ($Process.MainWindowHandle -ne 0) {
    Start-Sleep -Milliseconds 750
    Save-WindowScreenshot -WindowHandle $Process.MainWindowHandle -Path $Path
  }

  return [pscustomobject]@{
    delayMilliseconds = $DelayMilliseconds
    ashProcessId = $AshProcessId
    ashExited = $true
    screenshot = if ($Process.MainWindowHandle -ne 0) { $Path } else { $null }
  }
}

function Invoke-FrontendSmoke {
  param(
    [System.Diagnostics.Process]$Process,
    [string]$EvidencePath,
    [string]$ScreenshotPath,
    [string]$TranscriptPath,
    [string]$ExpectedOutput,
    [int]$TimeoutSeconds
  )

  $evidence = Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "frontend smoke evidence did not appear within ${TimeoutSeconds}s: $EvidencePath" `
    -Condition {
      if (-not (Test-Path -LiteralPath $EvidencePath -PathType Leaf)) { return $false }
      $content = Get-Content -LiteralPath $EvidencePath -Raw -ErrorAction SilentlyContinue
      if (-not $content) { return $false }
      try {
        return $content | ConvertFrom-Json
      } catch {
        return $false
      }
    }

  if ($evidence.status -ne 'passed') {
    throw "frontend smoke failed: $($evidence | ConvertTo-Json -Depth 6)"
  }
  if (-not $evidence.selection.selected) {
    throw "frontend selection smoke did not select expected text"
  }
  if (-not $evidence.copy.copied) {
    throw "frontend copy smoke did not copy selected text"
  }
  if (-not $evidence.paste.dispatched) {
    throw "frontend paste smoke did not dispatch paste text"
  }
  if (-not $evidence.scrollback.scrolled -or -not $evidence.scrollback.firstMarkerRetained -or -not $evidence.scrollback.lastMarkerRetained) {
    throw "frontend scrollback smoke did not retain and scroll marker lines"
  }

  Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "frontend paste output did not appear within ${TimeoutSeconds}s: $ExpectedOutput" `
    -Condition {
      if (-not (Test-Path -LiteralPath $TranscriptPath -PathType Leaf)) { return $false }
      $content = Get-Content -LiteralPath $TranscriptPath -Raw -ErrorAction SilentlyContinue
      if ($content -and $content.Contains($ExpectedOutput)) { return $true }
      return $false
    } | Out-Null

  $Process.Refresh()
  if ($Process.MainWindowHandle -ne 0) {
    Start-Sleep -Milliseconds 750
    Save-WindowScreenshot -WindowHandle $Process.MainWindowHandle -Path $ScreenshotPath
    $evidence | Add-Member -NotePropertyName screenshot -NotePropertyValue $ScreenshotPath -Force
  }

  return $evidence
}

function Reset-SmokeDirectory {
  param(
    [string]$Path,
    [string[]]$AllowedRoots
  )

  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $matchedRoot = $null
  foreach ($root in $AllowedRoots) {
    $fullAllowedRoot = [System.IO.Path]::GetFullPath($root)
    if ($fullPath.StartsWith($fullAllowedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
      $matchedRoot = $fullAllowedRoot
      break
    }
  }
  if (-not $matchedRoot) {
    throw "smoke directory must stay under an allowed root: $fullPath"
  }
  if ([string]::IsNullOrWhiteSpace((Split-Path -Leaf $fullPath))) {
    throw "refusing to reset an unsafe smoke directory: $fullPath"
  }

  if (Test-Path -LiteralPath $fullPath) {
    Remove-Item -LiteralPath $fullPath -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
}

function Test-BinaryFileContainsText {
  param(
    [string]$Path,
    [string]$Needle
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    return $false
  }
  try {
    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
  } catch [System.IO.IOException] {
    return $false
  }
  try {
    $bytes = New-Object byte[] $stream.Length
    try {
      $read = $stream.Read($bytes, 0, $bytes.Length)
    } catch [System.IO.IOException] {
      return $false
    }
    if ($read -lt $bytes.Length) {
      if ($read -le 0) {
        return $false
      }
      $bytes = $bytes[0..($read - 1)]
    }
    $text = [System.Text.Encoding]::GetEncoding(28591).GetString($bytes)
    return $text.Contains($Needle)
  } finally {
    $stream.Dispose()
  }
}

function Test-AnyBinaryFileContainsText {
  param(
    [string[]]$Paths,
    [string]$Needle
  )

  foreach ($path in $Paths) {
    if (Test-BinaryFileContainsText -Path $path -Needle $Needle) {
      return $true
    }
  }
  return $false
}

function Initialize-AshIntegrationState {
  param(
    [string]$ConfigRoot,
    [string]$DataRoot,
    [string[]]$AllowedRoots
  )

  Reset-SmokeDirectory -Path $ConfigRoot -AllowedRoots $AllowedRoots
  Reset-SmokeDirectory -Path $DataRoot -AllowedRoots $AllowedRoots

  $configDir = Join-Path $ConfigRoot 'ai-terminal'
  New-Item -ItemType Directory -Force -Path $configDir | Out-Null
  $configPath = Join-Path $configDir 'config.toml'
  @'
[ai]
provider = "mock"
model = "gui-smoke-model"
'@ | Set-Content -LiteralPath $configPath -Encoding utf8

  return [pscustomobject]@{
    configPath = $configPath
    databasePath = Join-Path (Join-Path $DataRoot 'ai-terminal') 'ai-terminal.db'
  }
}

function Invoke-AshIntegrationSmoke {
  param(
    [System.Diagnostics.Process]$Process,
    [string]$EvidencePath,
    [string]$ScreenshotPath,
    [string]$TranscriptPath,
    [string]$ConfigRoot,
    [string]$DataRoot,
    [string]$DatabasePath,
    [string]$AiMarker,
    [string]$ExternalMarker,
    [string]$Commands,
    [int]$DelayMilliseconds,
    [int]$TimeoutSeconds
  )

  $transcriptEvidence = Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "ash integration output did not appear within ${TimeoutSeconds}s" `
    -Condition {
      if (-not (Test-Path -LiteralPath $TranscriptPath -PathType Leaf)) { return $false }
      $content = Get-Content -LiteralPath $TranscriptPath -Raw -ErrorAction SilentlyContinue
      if (-not $content) { return $false }
      $hasAi = $content.Contains($AiMarker) -and $content.Contains('echo:')
      $hasBlocked = $content.Contains('rm -rf /') -and $content.Contains('Critical')
      $hasExternal = $content.Contains($ExternalMarker)
      if ($hasAi -and $hasBlocked -and $hasExternal) {
        return [pscustomobject]@{
          aiRouted = $true
          safetyGateBlocked = $true
          externalCommandRan = $true
        }
      }
      return $false
    }

  $dbEvidence = Wait-ForCondition `
    -TimeoutSeconds $TimeoutSeconds `
    -FailureMessage "ash integration database evidence did not appear within ${TimeoutSeconds}s: $DatabasePath" `
    -Condition {
      if (-not (Test-Path -LiteralPath $DatabasePath -PathType Leaf)) { return $false }
      $dbDir = Split-Path -Parent $DatabasePath
      $dbFiles = @(Get-ChildItem -LiteralPath $dbDir -Filter 'ai-terminal.db*' -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
      if ($dbFiles.Count -eq 0) { return $false }
      $hasUsage = Test-AnyBinaryFileContainsText -Paths $dbFiles -Needle 'gui-smoke-model'
      $hasCommand = Test-AnyBinaryFileContainsText -Paths $dbFiles -Needle $ExternalMarker
      $hasAudit = (Test-AnyBinaryFileContainsText -Paths $dbFiles -Needle 'command_blocked') -and
        (Test-AnyBinaryFileContainsText -Paths $dbFiles -Needle 'rm -rf /')
      if ($hasUsage -and $hasCommand -and $hasAudit) {
        return [pscustomobject]@{
          databasePath = $DatabasePath
          databaseFiles = $dbFiles
          usagePersisted = $true
          commandHistoryPersisted = $true
          auditBlockedPersisted = $true
        }
      }
      return $false
    }

  $Process.Refresh()
  if ($Process.MainWindowHandle -ne 0) {
    Start-Sleep -Milliseconds 750
    Save-WindowScreenshot -WindowHandle $Process.MainWindowHandle -Path $ScreenshotPath
  }

  $evidence = [pscustomobject]@{
    status = 'passed'
    delayMilliseconds = $DelayMilliseconds
    commands = @($Commands -split "`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    configRoot = $ConfigRoot
    dataRoot = $DataRoot
    transcript = $TranscriptPath
    transcriptEvidence = $transcriptEvidence
    database = $dbEvidence
    screenshot = if ($Process.MainWindowHandle -ne 0) { $ScreenshotPath } else { $null }
  }
  $evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
  return $evidence
}

$repoRoot = Split-Path $PSScriptRoot -Parent
if ([string]::IsNullOrWhiteSpace($PackageDir)) {
  $PackageDir = Resolve-DefaultPackageDir $repoRoot $PSScriptRoot
}
$PackageDir = (Resolve-Path -LiteralPath $PackageDir).Path

if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
  $EvidencePath = Join-Path $PackageDir 'gui-smoke-evidence.json'
}
if ([string]::IsNullOrWhiteSpace($ScreenshotPath)) {
  $ScreenshotPath = Join-Path $PackageDir 'gui-smoke-screenshot.png'
}
if ([string]::IsNullOrWhiteSpace($ResizeScreenshotPath)) {
  $ResizeScreenshotPath = Join-Path $PackageDir 'gui-smoke-resize-screenshot.png'
}
if ([string]::IsNullOrWhiteSpace($CtrlCScreenshotPath)) {
  $CtrlCScreenshotPath = Join-Path $PackageDir 'gui-smoke-ctrl-c-screenshot.png'
}
if ([string]::IsNullOrWhiteSpace($CtrlDScreenshotPath)) {
  $CtrlDScreenshotPath = Join-Path $PackageDir 'gui-smoke-ctrl-d-screenshot.png'
}
if ([string]::IsNullOrWhiteSpace($FrontendEvidencePath)) {
  $FrontendEvidencePath = Join-Path $PackageDir 'gui-smoke-frontend-evidence.json'
}
if ([string]::IsNullOrWhiteSpace($FrontendScreenshotPath)) {
  $FrontendScreenshotPath = Join-Path $PackageDir 'gui-smoke-frontend-screenshot.png'
}
if ([string]::IsNullOrWhiteSpace($AshIntegrationEvidencePath)) {
  $AshIntegrationEvidencePath = Join-Path $PackageDir 'gui-smoke-ash-integration-evidence.json'
}
if ([string]::IsNullOrWhiteSpace($AshIntegrationScreenshotPath)) {
  $AshIntegrationScreenshotPath = Join-Path $PackageDir 'gui-smoke-ash-integration-screenshot.png'
}
if ([string]::IsNullOrWhiteSpace($AshIntegrationConfigRoot)) {
  $AshIntegrationConfigRoot = Join-Path $PackageDir 'gui-smoke-config'
}
if ([string]::IsNullOrWhiteSpace($AshIntegrationDataRoot)) {
  $AshIntegrationDataRoot = Join-Path $PackageDir 'gui-smoke-data'
}
if ([string]::IsNullOrWhiteSpace($TranscriptPath)) {
  $TranscriptPath = Join-Path $PackageDir 'gui-smoke-transcript.txt'
}
$artifactsRoot = Join-Path $repoRoot 'artifacts'

$exe = Join-Path $PackageDir 'ai-terminal.exe'
$ash = Join-Path $PackageDir 'ash.exe'
$ai = Join-Path $PackageDir 'ai.exe'
Assert-File $exe
Assert-File $ash
Assert-File $ai
$ResolvedAshIntegrationCommands = $AshIntegrationCommands.Replace('{AI_EXE}', $ai)

if ($SkipChecksums) {
  $checksums = @()
} else {
  $checksums = Test-Sha256Manifest $PackageDir
}
$process = $null
$manualNotes = $null
$transcriptMatched = $false
$resizeEvidence = $null
$ctrlCEvidence = $null
$ctrlDEvidence = $null
$frontendEvidence = $null
$ashIntegrationEvidence = $null
$ashIntegrationState = $null

try {
  if (-not $SkipCommandSmoke -and (Test-Path -LiteralPath $TranscriptPath)) {
    Remove-Item -LiteralPath $TranscriptPath -Force
  }
  if (-not $SkipFrontendSmoke -and (Test-Path -LiteralPath $FrontendEvidencePath)) {
    Remove-Item -LiteralPath $FrontendEvidencePath -Force
  }
  if (-not $SkipAshIntegrationSmoke -and (Test-Path -LiteralPath $AshIntegrationEvidencePath)) {
    Remove-Item -LiteralPath $AshIntegrationEvidencePath -Force
  }
  if (-not $SkipAshIntegrationSmoke) {
    $ashIntegrationState = Initialize-AshIntegrationState `
      -ConfigRoot $AshIntegrationConfigRoot `
      -DataRoot $AshIntegrationDataRoot `
      -AllowedRoots @($PackageDir, $artifactsRoot)
  }

  $startEnvironment = @{
    AI_TERMINAL_ASH_PATH = $ash
  }
  if (-not $SkipAshIntegrationSmoke) {
    if ($SkipCommandSmoke) {
      throw "ash integration smoke requires command transcript; use -SkipAshIntegrationSmoke with -SkipCommandSmoke"
    }
    if ($AshIntegrationDelayMilliseconds -lt 0 -or $AshIntegrationDelayMilliseconds -gt 60000) {
      throw "AshIntegrationDelayMilliseconds must be between 0 and 60000"
    }
    if ($AshIntegrationCommandIntervalMilliseconds -lt 50 -or $AshIntegrationCommandIntervalMilliseconds -gt 10000) {
      throw "AshIntegrationCommandIntervalMilliseconds must be between 50 and 10000"
    }
    $startEnvironment['XDG_CONFIG_HOME'] = $AshIntegrationConfigRoot
    $startEnvironment['XDG_DATA_HOME'] = $AshIntegrationDataRoot
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_DELAY_MS'] = [string]$AshIntegrationDelayMilliseconds
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_INTERVAL_MS'] = [string]$AshIntegrationCommandIntervalMilliseconds
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_COMMANDS'] = $ResolvedAshIntegrationCommands
  }
  if (-not $SkipCommandSmoke) {
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_COMMAND'] = $SmokeCommand
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_TRANSCRIPT'] = $TranscriptPath
  }
  if (-not $SkipCtrlCSmoke) {
    if ($SkipCommandSmoke) {
      throw "Ctrl-C smoke requires command transcript; use -SkipCtrlCSmoke with -SkipCommandSmoke"
    }
    if ($CtrlCDelayMilliseconds -lt 0 -or $CtrlCDelayMilliseconds -gt 60000) {
      throw "CtrlCDelayMilliseconds must be between 0 and 60000"
    }
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_CTRL_C_DELAY_MS'] = [string]$CtrlCDelayMilliseconds
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_CTRL_C_INPUT'] = $CtrlCInput
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_CTRL_C_RECOVERY_COMMAND'] = $CtrlCRecoveryCommand
  }
  if (-not $SkipCtrlDSmoke) {
    if ($CtrlDDelayMilliseconds -lt 0 -or $CtrlDDelayMilliseconds -gt 60000) {
      throw "CtrlDDelayMilliseconds must be between 0 and 60000"
    }
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_CTRL_D_DELAY_MS'] = [string]$CtrlDDelayMilliseconds
  }
  if (-not $SkipFrontendSmoke) {
    if ($SkipCommandSmoke) {
      throw "frontend smoke requires command transcript; use -SkipFrontendSmoke with -SkipCommandSmoke"
    }
    if ($FrontendDelayMilliseconds -lt 0 -or $FrontendDelayMilliseconds -gt 60000) {
      throw "FrontendDelayMilliseconds must be between 0 and 60000"
    }
    if ($FrontendScrollbackLines -lt 20 -or $FrontendScrollbackLines -gt 2000) {
      throw "FrontendScrollbackLines must be between 20 and 2000"
    }
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_FRONTEND_EVIDENCE'] = $FrontendEvidencePath
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_FRONTEND_DELAY_MS'] = [string]$FrontendDelayMilliseconds
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_SELECTION_TEXT'] = $FrontendSelectionText
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_PASTE_TEXT'] = $FrontendPasteText
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_PASTE_EXPECTED_OUTPUT'] = $FrontendExpectedOutput
    $startEnvironment['AI_TERMINAL_GUI_SMOKE_SCROLLBACK_LINES'] = [string]$FrontendScrollbackLines
  }
  $process = Start-Process -FilePath $exe -WorkingDirectory $PackageDir -Environment $startEnvironment -PassThru

  Wait-ForCondition `
    -TimeoutSeconds $StartupTimeoutSeconds `
    -FailureMessage "ai-terminal.exe did not create a visible main window within ${StartupTimeoutSeconds}s" `
    -Condition {
      $process.Refresh()
      if ($process.MainWindowHandle -ne 0) { return $true }
      return $false
    } | Out-Null

  Wait-ForCondition `
    -TimeoutSeconds $StartupTimeoutSeconds `
    -FailureMessage "ai-terminal.exe main window did not reach a capturable size within ${StartupTimeoutSeconds}s" `
    -Condition {
      $process.Refresh()
      if ($process.MainWindowHandle -eq 0) { return $false }
      $bounds = Get-WindowBounds -WindowHandle $process.MainWindowHandle
      if ($bounds -and $bounds.width -ge 400 -and $bounds.height -ge 300) { return $bounds }
      return $false
    } | Out-Null

  $ashChild = Wait-ForCondition `
    -TimeoutSeconds $StartupTimeoutSeconds `
    -FailureMessage "ash.exe child process did not appear within ${StartupTimeoutSeconds}s" `
    -Condition {
      $descendants = Get-DescendantProcessInfo -RootProcessId $process.Id
      return $descendants | Where-Object { $_.Name -ieq 'ash.exe' } | Select-Object -First 1
    }

  $descendants = @(Get-DescendantProcessInfo -RootProcessId $process.Id)
  $unexpectedShells = @($descendants | Where-Object {
    $_.Name -in @('WindowsTerminal.exe', 'wt.exe', 'powershell.exe', 'pwsh.exe', 'cmd.exe')
  })
  if ($unexpectedShells.Count -gt 0) {
    throw "unexpected external shell descendant(s): $($unexpectedShells.Name -join ', ')"
  }

  if (-not $SkipCommandSmoke -and -not [string]::IsNullOrWhiteSpace($ExpectedOutput)) {
    Wait-ForCondition `
      -TimeoutSeconds $StartupTimeoutSeconds `
      -FailureMessage "smoke command output did not appear within ${StartupTimeoutSeconds}s: $ExpectedOutput" `
      -Condition {
        if (-not (Test-Path -LiteralPath $TranscriptPath -PathType Leaf)) { return $false }
        $content = Get-Content -LiteralPath $TranscriptPath -Raw -ErrorAction SilentlyContinue
        if ($content -and $content.Contains($ExpectedOutput)) { return $true }
        return $false
      } | Out-Null
    if (-not [string]::IsNullOrWhiteSpace($UnexpectedOutput)) {
      $content = Get-Content -LiteralPath $TranscriptPath -Raw -ErrorAction SilentlyContinue
      if ($content -and $content.Contains($UnexpectedOutput)) {
        throw "smoke command transcript contained unexpected output: $UnexpectedOutput"
      }
    }
    $transcriptMatched = $true
  }

  if (-not $SkipCtrlCSmoke) {
    $ctrlCTimeoutSeconds = $StartupTimeoutSeconds + [int][Math]::Ceiling($CtrlCDelayMilliseconds / 1000.0) + 5
    $ctrlCEvidence = Invoke-CtrlCSmoke `
      -Process $process `
      -AshProcessId ([int]$ashChild.ProcessId) `
      -Path $CtrlCScreenshotPath `
      -TranscriptPath $TranscriptPath `
      -PendingInput $CtrlCInput `
      -ExpectedOutput $CtrlCExpectedOutput `
      -DelayMilliseconds $CtrlCDelayMilliseconds `
      -TimeoutSeconds $ctrlCTimeoutSeconds
  }
  if (-not $SkipFrontendSmoke) {
    $frontendTimeoutSeconds = $StartupTimeoutSeconds + [int][Math]::Ceiling($FrontendDelayMilliseconds / 1000.0) + 5
    $frontendEvidence = Invoke-FrontendSmoke `
      -Process $process `
      -EvidencePath $FrontendEvidencePath `
      -ScreenshotPath $FrontendScreenshotPath `
      -TranscriptPath $TranscriptPath `
      -ExpectedOutput $FrontendExpectedOutput `
      -TimeoutSeconds $frontendTimeoutSeconds
  }
  if (-not $SkipAshIntegrationSmoke) {
    $ashCommandCount = @($ResolvedAshIntegrationCommands -split "`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }).Count
    $ashIntegrationTimeoutSeconds = (
      $StartupTimeoutSeconds +
      [int][Math]::Ceiling($AshIntegrationDelayMilliseconds / 1000.0) +
      [int][Math]::Ceiling(($AshIntegrationCommandIntervalMilliseconds * [Math]::Max(1, $ashCommandCount)) / 1000.0) +
      10
    )
    $ashIntegrationEvidence = Invoke-AshIntegrationSmoke `
      -Process $process `
      -EvidencePath $AshIntegrationEvidencePath `
      -ScreenshotPath $AshIntegrationScreenshotPath `
      -TranscriptPath $TranscriptPath `
      -ConfigRoot $AshIntegrationConfigRoot `
      -DataRoot $AshIntegrationDataRoot `
      -DatabasePath $ashIntegrationState.databasePath `
      -AiMarker $AshIntegrationAiMarker `
      -ExternalMarker $AshIntegrationExternalMarker `
      -Commands $ResolvedAshIntegrationCommands `
      -DelayMilliseconds $AshIntegrationDelayMilliseconds `
      -TimeoutSeconds $ashIntegrationTimeoutSeconds
  }

  Initialize-ScreenshotCapture
  [AiTerminalSmoke.NativeMethods]::SetForegroundWindow($process.MainWindowHandle) | Out-Null
  Start-Sleep -Milliseconds 750
  Save-WindowScreenshot -WindowHandle $process.MainWindowHandle -Path $ScreenshotPath
  if (-not $SkipResizeSmoke) {
    $resizeEvidence = Invoke-ResizeSmoke `
      -WindowHandle $process.MainWindowHandle `
      -Path $ResizeScreenshotPath `
      -TimeoutSeconds $StartupTimeoutSeconds
  }
  if (-not $SkipCtrlDSmoke) {
    $ctrlDTimeoutSeconds = $StartupTimeoutSeconds + [int][Math]::Ceiling($CtrlDDelayMilliseconds / 1000.0) + 5
    $ctrlDEvidence = Invoke-CtrlDSmoke `
      -Process $process `
      -AshProcessId ([int]$ashChild.ProcessId) `
      -Path $CtrlDScreenshotPath `
      -DelayMilliseconds $CtrlDDelayMilliseconds `
      -TimeoutSeconds $ctrlDTimeoutSeconds
  }

  if ($Interactive) {
    Write-Host ''
    Write-Host 'Manual GUI checks to perform in the AI Terminal window:'
    Write-Host '  1. Confirm no Windows Terminal, PowerShell, cmd, Git Bash, or MSYS window opened.'
    Write-Host '  2. Confirm the ash prompt is visible inside ai-terminal.exe.'
    Write-Host '  3. Run: [{size: 50} {size: 200}] | where size > 100'
    Write-Host '  4. Confirm output includes 200 and not 50.'
    Write-Host '  5. Resize the window and confirm terminal layout follows.'
    Write-Host '  6. Confirm AI routing, safety gate, storage/audit, and any exploratory UX not covered by automated evidence.'
    Write-Host ''
    $manualNotes = Read-Host 'Enter PASS or notes for the evidence file'
  }

  $process.Refresh()
  $evidence = [pscustomobject]@{
    status = 'running'
    timestamp = (Get-Date).ToString('o')
    packageDir = $PackageDir
    executable = $exe
    processId = $process.Id
    mainWindowHandle = $process.MainWindowHandle
    mainWindowTitle = $process.MainWindowTitle
    ashProcessId = [int]$ashChild.ProcessId
    screenshot = $ScreenshotPath
    transcript = if ($SkipCommandSmoke) { $null } else { $TranscriptPath }
    smokeCommand = if ($SkipCommandSmoke) { $null } else { $SmokeCommand }
    expectedOutput = if ($SkipCommandSmoke) { $null } else { $ExpectedOutput }
    unexpectedOutput = if ($SkipCommandSmoke) { $null } else { $UnexpectedOutput }
    transcriptMatched = $transcriptMatched
    resize = if ($SkipResizeSmoke) { $null } else { $resizeEvidence }
    ctrlC = if ($SkipCtrlCSmoke) { $null } else { $ctrlCEvidence }
    frontend = if ($SkipFrontendSmoke) { $null } else { $frontendEvidence }
    ashIntegration = if ($SkipAshIntegrationSmoke) { $null } else { $ashIntegrationEvidence }
    ctrlD = if ($SkipCtrlDSmoke) { $null } else { $ctrlDEvidence }
    descendants = @($descendants | Select-Object ProcessId, ParentProcessId, Name, CommandLine)
    checksums = $checksums
    manualNotes = $manualNotes
  }

  Request-WindowClose -Process $process
  if (-not $process.WaitForExit(45000)) {
    $process.Refresh()
    if ($process.MainWindowHandle -ne 0) {
      Initialize-ScreenshotCapture
      [AiTerminalSmoke.NativeMethods]::PostMessage(
        $process.MainWindowHandle,
        0x0010,
        [IntPtr]::Zero,
        [IntPtr]::Zero
      ) | Out-Null
    }
  }

  if (-not $process.WaitForExit(15000)) {
    throw "ai-terminal.exe did not exit after main window close"
  }

  Start-Sleep -Milliseconds 500
  $remainingAsh = @(Get-Process -Name 'ash' -ErrorAction SilentlyContinue | Where-Object { $_.Id -eq [int]$ashChild.ProcessId })
  if ($remainingAsh.Count -gt 0) {
    throw "ash.exe child remained after app close: pid $($ashChild.ProcessId)"
  }

  $evidence.status = 'passed'
  $evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
  Write-Output "GUI_SMOKE_OK $EvidencePath"
} catch {
  if ($process -and -not $process.HasExited) {
    $descendants = @(Get-DescendantProcessInfo -RootProcessId $process.Id | Sort-Object ProcessId -Descending)
    foreach ($child in $descendants) {
      Stop-Process -Id ([int]$child.ProcessId) -Force -ErrorAction SilentlyContinue
    }
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
  }

  $failure = [pscustomobject]@{
    status = 'failed'
    timestamp = (Get-Date).ToString('o')
    packageDir = $PackageDir
    error = $_.Exception.Message
  }
  $failure | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
  throw
}
