#!/usr/bin/env bash
# AI Terminal 설치 — GitHub Release 에서 Linux x86_64 바이너리를 받아 체크섬 검증 후 설치.
# 사용: curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | bash
#   환경변수: AI_VERSION(기본 latest), AI_INSTALL_DIR(기본 ~/.local/bin)
set -euo pipefail

REPO="ai-cli-terminal/terminal"
VERSION="${AI_VERSION:-latest}"
INSTALL_DIR="${AI_INSTALL_DIR:-$HOME/.local/bin}"
ASSET="ai-linux-x86_64"

if [ "$VERSION" = "latest" ]; then
  BASE="https://github.com/$REPO/releases/latest/download"
else
  BASE="https://github.com/$REPO/releases/download/$VERSION"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "downloading $ASSET ($VERSION)..."
curl -fsSL "$BASE/$ASSET" -o "$tmp/ai"
curl -fsSL "$BASE/$ASSET.sha256" -o "$tmp/ai.sha256"

echo "verifying checksum..."
expected="$(cut -d' ' -f1 "$tmp/ai.sha256" | tr -d '\r')"
if [ "${#expected}" -ne 64 ]; then
  echo "오류: 체크섬 파일이 손상되었습니다(64자 SHA256 아님)." >&2
  exit 1
fi
echo "$expected  $tmp/ai" | sha256sum -c -

mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp/ai" "$INSTALL_DIR/ai"
echo "installed: $INSTALL_DIR/ai"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "주의: $INSTALL_DIR 가 PATH 에 없습니다. 셸 rc 에 다음을 추가하세요:"; echo "  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
"$INSTALL_DIR/ai" --version
