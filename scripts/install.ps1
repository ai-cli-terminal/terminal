# AI Terminal 설치 (Windows) — GitHub Release 에서 ai.exe 를 받아 SHA256 검증 후 설치.
# 사용: irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
#   환경변수: AI_VERSION(기본 latest), AI_INSTALL_DIR(기본 $env:LOCALAPPDATA\Programs\ai-terminal)
$ErrorActionPreference = 'Stop'

$repo = 'ai-cli-terminal/terminal'
$version = if ($env:AI_VERSION) { $env:AI_VERSION } else { 'latest' }
$installDir = if ($env:AI_INSTALL_DIR) { $env:AI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\ai-terminal' }
$asset = 'ai-windows-x86_64.exe'

$base = if ($version -eq 'latest') {
  "https://github.com/$repo/releases/latest/download"
} else {
  "https://github.com/$repo/releases/download/$version"
}

$tmp = (New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid().ToString()))).FullName
try {
  Write-Output "downloading $asset ($version)..."
  Invoke-WebRequest "$base/$asset" -OutFile (Join-Path $tmp 'ai.exe') -UseBasicParsing
  Invoke-WebRequest "$base/$asset.sha256" -OutFile (Join-Path $tmp 'ai.exe.sha256') -UseBasicParsing

  Write-Output 'verifying checksum...'
  $expected = (Get-Content (Join-Path $tmp 'ai.exe.sha256') -Raw).Trim().Split(' ')[0].ToLower()
  if ($expected.Length -ne 64) { throw "sha256 파일이 손상되었습니다(64자 SHA256 아님)." }
  $actual = (Get-FileHash (Join-Path $tmp 'ai.exe') -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) { throw "checksum mismatch: expected $expected got $actual" }

  New-Item -ItemType Directory -Force -Path $installDir | Out-Null
  Copy-Item (Join-Path $tmp 'ai.exe') (Join-Path $installDir 'ai.exe') -Force
  Write-Output "installed: $installDir\ai.exe"
  if (-not ($env:Path -split ';' | Where-Object { $_ -eq $installDir })) {
    Write-Output "주의: $installDir 가 PATH 에 없습니다. 다음으로 영구 추가하세요:"
    Write-Output "  setx PATH `"$installDir;%PATH%`""
  }
  & (Join-Path $installDir 'ai.exe') --version
} finally {
  Remove-Item -Recurse -Force $tmp
}
