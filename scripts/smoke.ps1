# Windows 네이티브 C-free 빌드 + 핵심 명령 스모크.
# 사용: pwsh scripts/smoke.ps1
$ErrorActionPreference = 'Stop'

# 어느 디렉터리에서 실행하든 repo 루트 기준으로 동작.
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location $repoRoot

cargo build --release --features remote
if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }

$bin = Join-Path $repoRoot 'target\release\ai.exe'
if (-not (Test-Path $bin)) { throw "binary not found: $bin" }

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

Write-Output 'SMOKE_OK'
