#!/usr/bin/env bash
# AI Terminal 설치 — GitHub Release 에서 Linux x86_64 바이너리를 받아 검증 후 설치.
# 사용: curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | bash
#   환경변수: AI_VERSION(기본 latest), AI_INSTALL_DIR(기본 ~/.local/bin)
set -euo pipefail

REPO="ai-cli-terminal/terminal"
VERSION="${AI_VERSION:-latest}"
INSTALL_DIR="${AI_INSTALL_DIR:-$HOME/.local/bin}"
AI_ASSET="ai-linux-x86_64"
ASH_ASSET="ash-linux-x86_64"
MANIFEST_PAYLOAD="binary-manifest.json"
MANIFEST_SIGNATURE="binary-manifest.manifest.json"
MANIFEST_VERSION_FILE="$INSTALL_DIR/.ai-terminal-release-manifest-version"
SIGNED_MANIFEST_REQUIRED=0
MANIFEST_AVAILABLE=0
MANIFEST_VERIFIED=0
VERIFIED_MANIFEST_VERSION=""

if [ "$VERSION" = "latest" ]; then
  BASE="https://github.com/$REPO/releases/latest/download"
else
  BASE="https://github.com/$REPO/releases/download/$VERSION"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

if is_truthy "${AI_REQUIRE_SIGNED_MANIFEST:-0}"; then
  SIGNED_MANIFEST_REQUIRED=1
fi

download_signed_manifest() {
  echo "downloading signed binary manifest ($VERSION)..."
  if curl -fsSL "$BASE/$MANIFEST_PAYLOAD" -o "$tmp/$MANIFEST_PAYLOAD"; then
    if curl -fsSL "$BASE/$MANIFEST_SIGNATURE" -o "$tmp/$MANIFEST_SIGNATURE"; then
      MANIFEST_AVAILABLE=1
      return 0
    fi
  fi

  rm -f "$tmp/$MANIFEST_PAYLOAD" "$tmp/$MANIFEST_SIGNATURE"
  if [ "$SIGNED_MANIFEST_REQUIRED" -eq 1 ]; then
    echo "오류: signed binary manifest asset이 없어 설치를 중단합니다." >&2
    echo "      필요한 자산: $MANIFEST_PAYLOAD, $MANIFEST_SIGNATURE" >&2
    return 1
  fi
  echo "주의: signed binary manifest asset을 찾지 못해 checksum 검증만 사용합니다." >&2
}

select_manifest_verifier() {
  if [ -n "${AI_MANIFEST_VERIFIER:-}" ]; then
    if [ ! -x "$AI_MANIFEST_VERIFIER" ]; then
      echo "오류: AI_MANIFEST_VERIFIER 실행 파일을 찾을 수 없습니다: $AI_MANIFEST_VERIFIER" >&2
      return 1
    fi
    printf '%s\n' "$AI_MANIFEST_VERIFIER"
    return 0
  fi
  if [ -x "$INSTALL_DIR/ai" ]; then
    printf '%s\n' "$INSTALL_DIR/ai"
    return 0
  fi
  if command -v ai >/dev/null 2>&1; then
    command -v ai
    return 0
  fi
  return 1
}

compare_manifest_version() {
  local candidate="$1"
  local minimum="$2"
  local source="$3"
  if ! printf '%s' "$candidate" | grep -Eq '^[0-9]+$'; then
    echo "오류: signed manifest version이 숫자가 아닙니다: $candidate" >&2
    return 1
  fi
  if ! printf '%s' "$minimum" | grep -Eq '^[0-9]+$'; then
    echo "오류: $source manifest version이 숫자가 아닙니다: $minimum" >&2
    return 1
  fi
  if (( 10#$candidate < 10#$minimum )); then
    echo "오류: signed manifest downgrade 차단: candidate=$candidate minimum=$minimum ($source)" >&2
    return 1
  fi
}

enforce_manifest_version() {
  local version="$1"
  if [ -n "${AI_MIN_MANIFEST_VERSION:-}" ]; then
    compare_manifest_version "$version" "$AI_MIN_MANIFEST_VERSION" "AI_MIN_MANIFEST_VERSION" || return 1
  fi
  if [ -f "$MANIFEST_VERSION_FILE" ]; then
    local installed_version
    installed_version="$(tr -d '\r\n ' < "$MANIFEST_VERSION_FILE")"
    if [ -n "$installed_version" ]; then
      compare_manifest_version "$version" "$installed_version" "$MANIFEST_VERSION_FILE" || return 1
    fi
  fi
}

verify_signed_manifest_for_asset() {
  local asset="$1"
  local out="$2"
  local verifier
  local verify_output
  local manifest_version

  if [ "$MANIFEST_AVAILABLE" -ne 1 ]; then
    return 0
  fi

  if ! verifier="$(select_manifest_verifier)"; then
    if [ -n "${AI_MANIFEST_VERIFIER:-}" ]; then
      echo "오류: AI_MANIFEST_VERIFIER 설정이 잘못되어 설치를 중단합니다." >&2
      return 1
    fi
    if [ "$SIGNED_MANIFEST_REQUIRED" -eq 1 ]; then
      echo "오류: signed manifest 검증용 trust-enabled ai를 찾을 수 없습니다." >&2
      echo "      기존 설치본 또는 AI_MANIFEST_VERIFIER를 제공하세요." >&2
      return 1
    fi
    echo "주의: signed manifest 검증기를 찾지 못해 $asset 은 checksum 검증만 사용합니다." >&2
    return 0
  fi

  echo "verifying $asset signed manifest..."
  if ! verify_output="$("$verifier" release manifest verify \
    --payload "$tmp/$MANIFEST_PAYLOAD" \
    --manifest "$tmp/$MANIFEST_SIGNATURE" \
    --name "$asset" \
    --artifact "$tmp/$out" 2>&1)"; then
    printf '%s\n' "$verify_output" >&2
    if [ "$SIGNED_MANIFEST_REQUIRED" -eq 1 ] ||
       [ -n "${AI_MANIFEST_VERIFIER:-}" ] ||
       [ -n "${AI_TERMINAL_ORG_TRUST_ANCHOR:-}" ] ||
       [ "$MANIFEST_VERIFIED" -eq 1 ]; then
      echo "오류: signed binary manifest 검증 실패." >&2
      return 1
    fi
    echo "주의: signed manifest 검증에 실패해 $asset 은 checksum 검증만 사용합니다." >&2
    return 0
  fi

  printf '%s\n' "$verify_output"
  manifest_version="$(printf '%s\n' "$verify_output" | awk -F: '/^version[[:space:]]*:/ { gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit }')"
  if [ -z "$manifest_version" ]; then
    echo "오류: signed manifest 검증 결과에서 version을 찾지 못했습니다." >&2
    return 1
  fi
  enforce_manifest_version "$manifest_version" || return 1
  VERIFIED_MANIFEST_VERSION="$manifest_version"
  MANIFEST_VERIFIED=1
}

download_and_verify() {
  local asset="$1"
  local out="$2"

  echo "downloading $asset ($VERSION)..."
  curl -fsSL "$BASE/$asset" -o "$tmp/$out" || return 1
  curl -fsSL "$BASE/$asset.sha256" -o "$tmp/$out.sha256" || return 1

  echo "verifying $asset checksum..."
  local expected
  expected="$(cut -d' ' -f1 "$tmp/$out.sha256" | tr -d '\r')"
  if [ "${#expected}" -ne 64 ]; then
    echo "오류: 체크섬 파일이 손상되었습니다(64자 SHA256 아님)." >&2
    return 1
  fi
  echo "$expected  $tmp/$out" | sha256sum -c - || return 1
  verify_signed_manifest_for_asset "$asset" "$out" || return 1
}

download_signed_manifest || exit 1
download_and_verify "$AI_ASSET" "ai" || exit 1
echo "downloading $ASH_ASSET ($VERSION)..."
if curl -fsSL "$BASE/$ASH_ASSET" -o "$tmp/ash"; then
  curl -fsSL "$BASE/$ASH_ASSET.sha256" -o "$tmp/ash.sha256"
  echo "verifying $ASH_ASSET checksum..."
  expected="$(cut -d' ' -f1 "$tmp/ash.sha256" | tr -d '\r')"
  if [ "${#expected}" -ne 64 ]; then
    echo "오류: 체크섬 파일이 손상되었습니다(64자 SHA256 아님)." >&2
    exit 1
  fi
  echo "$expected  $tmp/ash" | sha256sum -c -
  verify_signed_manifest_for_asset "$ASH_ASSET" "ash"
  HAS_ASH=1
else
  HAS_ASH=0
  echo "주의: 이 릴리즈에는 ash-linux-x86_64 asset이 없어 ai만 설치합니다." >&2
fi

mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp/ai" "$INSTALL_DIR/ai"
if [ "$HAS_ASH" -eq 1 ]; then
  install -m 0755 "$tmp/ash" "$INSTALL_DIR/ash"
fi
if [ "$MANIFEST_VERIFIED" -eq 1 ]; then
  printf '%s\n' "$VERIFIED_MANIFEST_VERSION" > "$MANIFEST_VERSION_FILE"
  echo "signed manifest version: $VERIFIED_MANIFEST_VERSION"
fi
echo "installed: $INSTALL_DIR/ai"
if [ "$HAS_ASH" -eq 1 ]; then
  echo "installed: $INSTALL_DIR/ash"
fi
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "주의: $INSTALL_DIR 가 PATH 에 없습니다. 셸 rc 에 다음을 추가하세요:"; echo "  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
"$INSTALL_DIR/ai" --version
