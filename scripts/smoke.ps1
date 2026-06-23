# Windows 네이티브 C-free 빌드 + 핵심 명령 스모크.
# 사용: pwsh scripts/smoke.ps1
$ErrorActionPreference = 'Stop'

# 어느 디렉터리에서 실행하든 repo 루트 기준으로 동작.
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location $repoRoot

cargo build --release --bins --features remote
if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }

$bin = Join-Path $repoRoot 'target\release\ai.exe'
if (-not (Test-Path $bin)) { throw "binary not found: $bin" }
$ash = Join-Path $repoRoot 'target\release\ash.exe'
if (-not (Test-Path $ash)) { throw "binary not found: $ash" }

& $bin --version
if ($LASTEXITCODE -ne 0) { throw "ai --version failed (exit $LASTEXITCODE)" }
& $bin doctor
if ($LASTEXITCODE -ne 0) { throw "ai doctor failed (exit $LASTEXITCODE)" }
& $bin risk "rm -rf /tmp/x"
if ($LASTEXITCODE -ne 0) { throw "ai risk failed (exit $LASTEXITCODE)" }
& $bin mask "token=ghp_0123456789abcdef0123456789abcdef0123"
if ($LASTEXITCODE -ne 0) { throw "ai mask failed (exit $LASTEXITCODE)" }
& $bin preview "rm -rf ./build"
if ($LASTEXITCODE -ne 0) { throw "ai preview failed (exit $LASTEXITCODE)" }

$core = "[{size: 50} {size: 200}] | where size > 100`nexit`n" | & $ash 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { throw "ash core smoke failed (exit $LASTEXITCODE): $core" }
if ($core -notmatch '\b200\b' -or $core -match '\b50\b') { throw "ash core smoke output mismatch: $core" }

$tmp = New-Item -ItemType Directory -Force -Path (Join-Path ([System.IO.Path]::GetTempPath()) 'ash-smoke')
Set-Content -Path (Join-Path $tmp 'ash-smoke.cmd') -Encoding ascii -Value "@echo off`r`necho CMD_OK`r`nexit /b 0`r`n"
Set-Content -Path (Join-Path $tmp 'ash-ps.ps1') -Encoding ascii -Value "Write-Output 'PS_OK'`r`nexit 0`r`n"

Push-Location $tmp
try {
  $cmdOut = "ash-smoke`nexit`n" | & $ash 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0 -or $cmdOut -notmatch 'CMD_OK') { throw "ash .cmd smoke failed: $cmdOut" }
  $psOut = "ash-ps.ps1`nexit`n" | & $ash 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0 -or $psOut -notmatch 'PS_OK') { throw "ash .ps1 smoke failed: $psOut" }
} finally {
  Pop-Location
}

Write-Output 'SMOKE_OK'
