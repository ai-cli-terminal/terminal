# AI Terminal 설치 (Windows) — GitHub Release 에서 ai.exe 와 ash.exe 를 받아 검증 후 설치.
# 사용: irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
#   환경변수: AI_VERSION(기본 latest), AI_INSTALL_DIR(기본 $env:LOCALAPPDATA\Programs\ai-terminal)
$ErrorActionPreference = 'Stop'

$repo = 'ai-cli-terminal/terminal'
$version = if ($env:AI_VERSION) { $env:AI_VERSION } else { 'latest' }
$installDir = if ($env:AI_INSTALL_DIR) { $env:AI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\ai-terminal' }
$aiAsset = 'ai-windows-x86_64.exe'
$ashAsset = 'ash-windows-x86_64.exe'
$manifestPayload = 'binary-manifest.json'
$manifestSignature = 'binary-manifest.manifest.json'
$manifestVersionFile = Join-Path $installDir '.ai-terminal-release-manifest-version'
$manifestAvailable = $false
$manifestVerified = $false
$verifiedManifestVersion = $null

$base = if ($version -eq 'latest') {
  "https://github.com/$repo/releases/latest/download"
} else {
  "https://github.com/$repo/releases/download/$version"
}

$tmp = (New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid().ToString()))).FullName
try {
  function Test-Truthy($value) {
    if (-not $value) { return $false }
    return @('1', 'true', 'yes', 'on') -contains $value.ToString().ToLowerInvariant()
  }

  $signedManifestRequired = Test-Truthy $env:AI_REQUIRE_SIGNED_MANIFEST

  function Download-SignedManifest {
    Write-Output "downloading signed binary manifest ($version)..."
    try {
      Invoke-WebRequest "$base/$manifestPayload" -OutFile (Join-Path $tmp $manifestPayload) -UseBasicParsing
      Invoke-WebRequest "$base/$manifestSignature" -OutFile (Join-Path $tmp $manifestSignature) -UseBasicParsing
      $script:manifestAvailable = $true
    } catch {
      Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $tmp $manifestPayload), (Join-Path $tmp $manifestSignature)
      if ($signedManifestRequired) {
        throw "signed binary manifest asset이 없어 설치를 중단합니다. 필요한 자산: $manifestPayload, $manifestSignature"
      }
      Write-Warning "signed binary manifest asset을 찾지 못해 checksum 검증만 사용합니다."
    }
  }

  function Resolve-ManifestVerifier {
    if ($env:AI_MANIFEST_VERIFIER) {
      if (-not (Test-Path -LiteralPath $env:AI_MANIFEST_VERIFIER -PathType Leaf)) {
        throw "AI_MANIFEST_VERIFIER 실행 파일을 찾을 수 없습니다: $env:AI_MANIFEST_VERIFIER"
      }
      return $env:AI_MANIFEST_VERIFIER
    }
    $installed = Join-Path $installDir 'ai.exe'
    if (Test-Path -LiteralPath $installed -PathType Leaf) {
      return $installed
    }
    $pathCommand = Get-Command 'ai.exe' -ErrorAction SilentlyContinue
    if ($pathCommand) {
      return $pathCommand.Source
    }
    return $null
  }

  function Assert-ManifestVersion($candidate, $minimum, $source) {
    if ($candidate -notmatch '^\d+$') { throw "signed manifest version이 숫자가 아닙니다: $candidate" }
    if ($minimum -notmatch '^\d+$') { throw "$source manifest version이 숫자가 아닙니다: $minimum" }
    if ([UInt64]$candidate -lt [UInt64]$minimum) {
      throw "signed manifest downgrade 차단: candidate=$candidate minimum=$minimum ($source)"
    }
  }

  function Enforce-ManifestVersion($candidate) {
    if ($env:AI_MIN_MANIFEST_VERSION) {
      Assert-ManifestVersion $candidate $env:AI_MIN_MANIFEST_VERSION 'AI_MIN_MANIFEST_VERSION'
    }
    if (Test-Path -LiteralPath $manifestVersionFile -PathType Leaf) {
      $installedVersion = (Get-Content -LiteralPath $manifestVersionFile -Raw).Trim()
      if ($installedVersion) {
        Assert-ManifestVersion $candidate $installedVersion $manifestVersionFile
      }
    }
  }

  function Invoke-SignedManifestVerification($asset, $outFile) {
    if (-not $script:manifestAvailable) { return }

    $verifier = Resolve-ManifestVerifier
    if (-not $verifier) {
      if ($signedManifestRequired) {
        throw "signed manifest 검증용 trust-enabled ai를 찾을 수 없습니다. 기존 설치본 또는 AI_MANIFEST_VERIFIER를 제공하세요."
      }
      Write-Warning "signed manifest 검증기를 찾지 못해 $asset 은 checksum 검증만 사용합니다."
      return
    }

    Write-Output "verifying $asset signed manifest..."
    $arguments = @(
      'release', 'manifest', 'verify',
      '--payload', (Join-Path $tmp $manifestPayload),
      '--manifest', (Join-Path $tmp $manifestSignature),
      '--name', $asset,
      '--artifact', (Join-Path $tmp $outFile)
    )
    $verifyOutput = & $verifier @arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
      $text = ($verifyOutput | Out-String).Trim()
      if ($text) { Write-Warning $text }
      if ($signedManifestRequired -or $env:AI_MANIFEST_VERIFIER -or $env:AI_TERMINAL_ORG_TRUST_ANCHOR -or $script:manifestVerified) {
        throw "signed binary manifest 검증 실패."
      }
      Write-Warning "signed manifest 검증에 실패해 $asset 은 checksum 검증만 사용합니다."
      return
    }

    $verifyOutput | ForEach-Object { Write-Output $_ }
    $versionLine = $verifyOutput | Where-Object { $_ -match '^version\s*:' } | Select-Object -First 1
    $versionText = if ($versionLine) { $versionLine.ToString() } else { '' }
    if ($versionText -notmatch '^version\s*:\s*(\d+)\s*$') {
      throw "signed manifest 검증 결과에서 version을 찾지 못했습니다."
    }
    $manifestVersion = $Matches[1]
    Enforce-ManifestVersion $manifestVersion
    $script:verifiedManifestVersion = $manifestVersion
    $script:manifestVerified = $true
  }

  function Download-And-Verify($asset, $outFile) {
    Write-Output "downloading $asset ($version)..."
    Invoke-WebRequest "$base/$asset" -OutFile (Join-Path $tmp $outFile) -UseBasicParsing
    Invoke-WebRequest "$base/$asset.sha256" -OutFile (Join-Path $tmp "$outFile.sha256") -UseBasicParsing

    Write-Output "verifying $asset checksum..."
    $expected = (Get-Content (Join-Path $tmp "$outFile.sha256") -Raw).Trim().Split(' ')[0].ToLower()
    if ($expected.Length -ne 64) { throw "sha256 파일이 손상되었습니다(64자 SHA256 아님)." }
    $actual = (Get-FileHash (Join-Path $tmp $outFile) -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "checksum mismatch: expected $expected got $actual" }
    Invoke-SignedManifestVerification $asset $outFile
  }

  Download-SignedManifest
  Download-And-Verify $aiAsset 'ai.exe'
  $hasAsh = $true
  try {
    Write-Output "downloading $ashAsset ($version)..."
    Invoke-WebRequest "$base/$ashAsset" -OutFile (Join-Path $tmp 'ash.exe') -UseBasicParsing
  } catch {
    $hasAsh = $false
    Write-Warning "이 릴리즈에는 ash-windows-x86_64.exe asset이 없어 ai.exe만 설치합니다."
  }
  if ($hasAsh) {
    Invoke-WebRequest "$base/$ashAsset.sha256" -OutFile (Join-Path $tmp 'ash.exe.sha256') -UseBasicParsing
    Write-Output "verifying $ashAsset checksum..."
    $expected = (Get-Content (Join-Path $tmp 'ash.exe.sha256') -Raw).Trim().Split(' ')[0].ToLower()
    if ($expected.Length -ne 64) { throw "sha256 파일이 손상되었습니다(64자 SHA256 아님)." }
    $actual = (Get-FileHash (Join-Path $tmp 'ash.exe') -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "checksum mismatch: expected $expected got $actual" }
    Invoke-SignedManifestVerification $ashAsset 'ash.exe'
  }

  New-Item -ItemType Directory -Force -Path $installDir | Out-Null
  Copy-Item (Join-Path $tmp 'ai.exe') (Join-Path $installDir 'ai.exe') -Force
  if ($hasAsh) {
    Copy-Item (Join-Path $tmp 'ash.exe') (Join-Path $installDir 'ash.exe') -Force
  }
  if ($manifestVerified) {
    Set-Content -LiteralPath $manifestVersionFile -Encoding ascii -Value $verifiedManifestVersion
    Write-Output "signed manifest version: $verifiedManifestVersion"
  }
  Write-Output "installed: $installDir\ai.exe"
  if ($hasAsh) {
    Write-Output "installed: $installDir\ash.exe"
  }
  if (-not ($env:Path -split ';' | Where-Object { $_ -eq $installDir })) {
    Write-Output "주의: $installDir 가 PATH 에 없습니다. 다음으로 영구 추가하세요:"
    Write-Output "  setx PATH `"$installDir;%PATH%`""
  }
  & (Join-Path $installDir 'ai.exe') --version
} finally {
  Remove-Item -Recurse -Force $tmp
}
