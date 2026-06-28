# F-Droid release activation smoke.
#
# This checks the transformation needed before submitting the fdroiddata draft:
# - replace TODO_NEXT_ANDROID_RELEASE_COMMIT with a full release commit hash
# - remove the temporary disabled build-block marker
# - run fdroid lint and rewritemeta on the activated metadata copy
#
# By default it does not modify source files. Use -Apply only when preparing the
# real fdroiddata submission after the Android release commit/tag exists.
param(
  [Parameter(Mandatory = $true)]
  [string]$Commit,
  [switch]$Apply,
  [string]$SmokeRoot = '',
  [string]$ToolRoot = '',
  [string]$AppId = 'dev.aiterminal.android',
  [string]$FdroidServerVersion = '2.4.5'
)

$ErrorActionPreference = 'Stop'

$androidDir = $PSScriptRoot
$repoRoot = Split-Path $androidDir -Parent
if ([string]::IsNullOrWhiteSpace($SmokeRoot)) {
  $SmokeRoot = Join-Path $repoRoot 'artifacts\fdroid-activation-smoke'
}
if ([string]::IsNullOrWhiteSpace($ToolRoot)) {
  $ToolRoot = Join-Path $repoRoot 'artifacts\fdroid-dry-run'
}

$resolvedSmokeRoot = [System.IO.Path]::GetFullPath($SmokeRoot)
$resolvedToolRoot = [System.IO.Path]::GetFullPath($ToolRoot)
$resolvedRepoRoot = [System.IO.Path]::GetFullPath($repoRoot)
if (-not $resolvedSmokeRoot.StartsWith($resolvedRepoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "SmokeRoot must stay under repo root: $resolvedRepoRoot"
}
if (-not $resolvedToolRoot.StartsWith($resolvedRepoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "ToolRoot must stay under repo root: $resolvedRepoRoot"
}
if ($Commit -notmatch '^[0-9a-fA-F]{40}$') {
  throw "Commit must be a full 40-character git commit hash"
}

$wsl = Get-Command wsl.exe -ErrorAction Stop
$metadataPath = Join-Path $repoRoot "android\fdroiddata\metadata\$AppId.yml"
if (-not (Test-Path -LiteralPath $metadataPath -PathType Leaf)) {
  throw "F-Droid metadata draft not found: $metadataPath"
}

New-Item -ItemType Directory -Force -Path $resolvedSmokeRoot | Out-Null
New-Item -ItemType Directory -Force -Path $resolvedToolRoot | Out-Null

$sourceMetadata = Get-Content -Raw -LiteralPath $metadataPath
if ($sourceMetadata -notmatch 'commit: TODO_NEXT_ANDROID_RELEASE_COMMIT') {
  throw "Metadata draft does not contain TODO_NEXT_ANDROID_RELEASE_COMMIT"
}
if ($sourceMetadata -notmatch '(?m)^\s+disable: Pending next Android release tag') {
  throw "Metadata draft does not contain the expected disabled build-block marker"
}

$activated = $sourceMetadata `
  -replace '(?m)^\s+disable: Pending next Android release tag that includes F-Droid metadata and fdroid-version\.properties\r?\n', '' `
  -replace 'commit: TODO_NEXT_ANDROID_RELEASE_COMMIT', "commit: $($Commit.ToLowerInvariant())" `
  -replace 'replace TODO_NEXT_ANDROID_RELEASE_COMMIT with the full\s+commit hash for the Android release tag', 'use the full commit hash for the Android release tag'

$activatedMetadataPath = Join-Path $resolvedSmokeRoot "$AppId.yml"
Set-Content -LiteralPath $activatedMetadataPath -Value $activated -Encoding utf8

function ConvertTo-WslPath([string]$Path) {
  $fullPath = [System.IO.Path]::GetFullPath($Path)
  if ($fullPath -match '^([A-Za-z]):\\(.*)$') {
    $drive = $Matches[1].ToLowerInvariant()
    $rest = $Matches[2].Replace('\', '/')
    return "/mnt/$drive/$rest"
  }
  $converted = & $wsl.Source -- wslpath -a $fullPath
  if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($converted)) {
    throw "wslpath failed for: $fullPath"
  }
  return $converted.Trim()
}

$repoRootWsl = ConvertTo-WslPath $resolvedRepoRoot
$smokeRootWsl = ConvertTo-WslPath $resolvedSmokeRoot
$toolRootWsl = ConvertTo-WslPath $resolvedToolRoot
$activatedMetadataWsl = ConvertTo-WslPath $activatedMetadataPath
$runnerPath = Join-Path $resolvedSmokeRoot 'run-fdroid-activation-smoke.sh'
$runnerPathWsl = ConvertTo-WslPath $runnerPath

$runner = @'
#!/usr/bin/env bash
set -euo pipefail

repo_root="$1"
smoke_root="$2"
tool_root="$3"
app_id="$4"
fdroid_server_version="$5"
activated_metadata="$6"

mkdir -p "$tool_root/tools" "$smoke_root/fdroiddata/metadata" "$smoke_root/fdroiddata/config"

if [ ! -f "$tool_root/tools/virtualenv.pyz" ]; then
  curl -L --fail --show-error --silent https://bootstrap.pypa.io/virtualenv.pyz -o "$tool_root/tools/virtualenv.pyz"
fi

if [ ! -x "$tool_root/venv/bin/fdroid" ]; then
  python3 "$tool_root/tools/virtualenv.pyz" "$tool_root/venv"
  "$tool_root/venv/bin/python" -m pip install "fdroidserver==$fdroid_server_version"
fi

actual_fdroid_version="$("$tool_root/venv/bin/fdroid" --version)"
if [ "$actual_fdroid_version" != "$fdroid_server_version" ]; then
  "$tool_root/venv/bin/python" -m pip install "fdroidserver==$fdroid_server_version"
  actual_fdroid_version="$("$tool_root/venv/bin/fdroid" --version)"
fi

cp "$activated_metadata" "$smoke_root/fdroiddata/metadata/$app_id.yml"

if [ ! -f "$smoke_root/fdroiddata/config/categories.yml" ]; then
  curl -L --fail --show-error --silent \
    https://gitlab.com/fdroid/fdroiddata/-/raw/master/config/categories.yml \
    -o "$smoke_root/fdroiddata/config/categories.yml"
fi

while read -r icon; do
  if [ -n "$icon" ] && [ ! -f "$smoke_root/fdroiddata/config/$icon" ]; then
    curl -L --fail --show-error --silent \
      "https://gitlab.com/fdroid/fdroiddata/-/raw/master/config/$icon" \
      -o "$smoke_root/fdroiddata/config/$icon"
  fi
done < <(awk '/^[[:space:]]+icon: / {print $2}' "$smoke_root/fdroiddata/config/categories.yml" | sort -u)

cd "$smoke_root/fdroiddata"

set +e
"$tool_root/venv/bin/fdroid" rewritemeta "$app_id" >"$smoke_root/fdroid-activation-rewritemeta.log" 2>&1
rewritemeta_status=$?
set -e
cat "$smoke_root/fdroid-activation-rewritemeta.log"
if [ "$rewritemeta_status" -ne 0 ]; then
  exit "$rewritemeta_status"
fi

set +e
"$tool_root/venv/bin/fdroid" lint "$app_id" >"$smoke_root/fdroid-activation-lint.log" 2>&1
lint_status=$?
set -e
cat "$smoke_root/fdroid-activation-lint.log"
if [ "$lint_status" -ne 0 ]; then
  exit "$lint_status"
fi

cp "metadata/$app_id.yml" "$smoke_root/$app_id.rewritten.yml"

cat > "$smoke_root/fdroid-activation-smoke-evidence.json" <<EOF
{
  "status": "passed",
  "appId": "$app_id",
  "fdroidServerVersion": "$actual_fdroid_version",
  "activatedMetadata": "$activated_metadata",
  "rewrittenMetadata": "$smoke_root/$app_id.rewritten.yml",
  "lintLog": "$smoke_root/fdroid-activation-lint.log",
  "rewritemetaLog": "$smoke_root/fdroid-activation-rewritemeta.log"
}
EOF

echo "FDROID_ACTIVATION_SMOKE_OK $smoke_root/fdroid-activation-smoke-evidence.json"
'@

Set-Content -LiteralPath $runnerPath -Value $runner -Encoding utf8
& $wsl.Source -- bash $runnerPathWsl $repoRootWsl $smokeRootWsl $toolRootWsl $AppId $FdroidServerVersion $activatedMetadataWsl
if ($LASTEXITCODE -ne 0) {
  throw "F-Droid activation smoke failed with exit code $LASTEXITCODE"
}

$rewrittenMetadataPath = Join-Path $resolvedSmokeRoot "$AppId.rewritten.yml"
if (-not (Test-Path -LiteralPath $rewrittenMetadataPath -PathType Leaf)) {
  throw "rewritten activated metadata was not generated: $rewrittenMetadataPath"
}

if ($Apply) {
  Copy-Item -LiteralPath $rewrittenMetadataPath -Destination $metadataPath -Force
  Write-Output "FDROID_ACTIVATION_APPLIED $metadataPath"
} else {
  Write-Output "FDROID_ACTIVATION_DRY_RUN $rewrittenMetadataPath"
}
