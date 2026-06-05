# Windows 네이티브 C-free 빌드 + 핵심 명령 스모크.
# 사용: pwsh scripts/smoke.ps1
$ErrorActionPreference = 'Stop'

cargo build --release --features remote
$bin = Join-Path (Get-Location) 'target\release\ai.exe'
if (-not (Test-Path $bin)) { throw "binary not found: $bin" }

& $bin --version
& $bin doctor
& $bin risk "rm -rf /tmp/x"
& $bin mask "token=ghp_0123456789abcdef0123456789abcdef0123"
& $bin preview "rm -rf ./build"
Write-Output 'SMOKE_OK'
