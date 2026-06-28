# F-Droid metadata smoke using fdroidserver in an isolated WSL virtualenv.
#
# This validates the fdroiddata draft with:
# - fdroid lint <appid>
# - fdroid rewritemeta <appid>
# - no source diff after rewritemeta
#
# It writes only under artifacts/fdroid-dry-run by default.
param(
  [string]$SmokeRoot = '',
  [string]$AppId = 'dev.aiterminal.android',
  [string]$FdroidServerVersion = '2.4.5'
)

$ErrorActionPreference = 'Stop'

$androidDir = $PSScriptRoot
$repoRoot = Split-Path $androidDir -Parent
if ([string]::IsNullOrWhiteSpace($SmokeRoot)) {
  $SmokeRoot = Join-Path $repoRoot 'artifacts\fdroid-dry-run'
}

$resolvedSmokeRoot = [System.IO.Path]::GetFullPath($SmokeRoot)
$resolvedRepoRoot = [System.IO.Path]::GetFullPath($repoRoot)
if (-not $resolvedSmokeRoot.StartsWith($resolvedRepoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "SmokeRoot must stay under repo root: $resolvedRepoRoot"
}

$wsl = Get-Command wsl.exe -ErrorAction Stop
$metadataPath = Join-Path $repoRoot "android\fdroiddata\metadata\$AppId.yml"
if (-not (Test-Path -LiteralPath $metadataPath -PathType Leaf)) {
  throw "F-Droid metadata draft not found: $metadataPath"
}

New-Item -ItemType Directory -Force -Path $resolvedSmokeRoot | Out-Null

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
$runnerPath = Join-Path $resolvedSmokeRoot 'run-fdroid-metadata-smoke.sh'
$runnerPathWsl = ConvertTo-WslPath $runnerPath

$runner = @'
#!/usr/bin/env bash
set -euo pipefail

repo_root="$1"
smoke_root="$2"
app_id="$3"
fdroid_server_version="$4"

cd "$repo_root"

mkdir -p "$smoke_root/tools" "$smoke_root/fdroiddata/metadata" "$smoke_root/fdroiddata/config"

if [ ! -f "$smoke_root/tools/virtualenv.pyz" ]; then
  curl -L --fail --show-error --silent https://bootstrap.pypa.io/virtualenv.pyz -o "$smoke_root/tools/virtualenv.pyz"
fi

if [ ! -x "$smoke_root/venv/bin/fdroid" ]; then
  python3 "$smoke_root/tools/virtualenv.pyz" "$smoke_root/venv"
  "$smoke_root/venv/bin/python" -m pip install "fdroidserver==$fdroid_server_version"
fi

actual_fdroid_version="$("$smoke_root/venv/bin/fdroid" --version)"
if [ "$actual_fdroid_version" != "$fdroid_server_version" ]; then
  "$smoke_root/venv/bin/python" -m pip install "fdroidserver==$fdroid_server_version"
  actual_fdroid_version="$("$smoke_root/venv/bin/fdroid" --version)"
fi

cp "android/fdroiddata/metadata/$app_id.yml" "$smoke_root/fdroiddata/metadata/$app_id.yml"

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
../venv/bin/fdroid lint "$app_id" >"$smoke_root/fdroid-lint.log" 2>&1
lint_status=$?
set -e
cat "$smoke_root/fdroid-lint.log"
if [ "$lint_status" -ne 0 ]; then
  exit "$lint_status"
fi

set +e
../venv/bin/fdroid rewritemeta "$app_id" >"$smoke_root/fdroid-rewritemeta.log" 2>&1
rewritemeta_status=$?
set -e
cat "$smoke_root/fdroid-rewritemeta.log"
if [ "$rewritemeta_status" -ne 0 ]; then
  exit "$rewritemeta_status"
fi

if ! cmp -s "metadata/$app_id.yml" "$repo_root/android/fdroiddata/metadata/$app_id.yml"; then
  diff -u "$repo_root/android/fdroiddata/metadata/$app_id.yml" "metadata/$app_id.yml" > "$smoke_root/fdroid-rewritemeta.diff" || true
  cat "$smoke_root/fdroid-rewritemeta.diff"
  exit 42
fi

cat > "$smoke_root/fdroid-metadata-smoke-evidence.json" <<EOF
{
  "status": "passed",
  "appId": "$app_id",
  "fdroidServerVersion": "$actual_fdroid_version",
  "metadata": "$repo_root/android/fdroiddata/metadata/$app_id.yml",
  "lintLog": "$smoke_root/fdroid-lint.log",
  "rewritemetaLog": "$smoke_root/fdroid-rewritemeta.log"
}
EOF

echo "FDROID_METADATA_SMOKE_OK $smoke_root/fdroid-metadata-smoke-evidence.json"
'@

Set-Content -LiteralPath $runnerPath -Value $runner -Encoding utf8
& $wsl.Source -- bash $runnerPathWsl $repoRootWsl $smokeRootWsl $AppId $FdroidServerVersion
if ($LASTEXITCODE -ne 0) {
  throw "F-Droid metadata smoke failed with exit code $LASTEXITCODE"
}
