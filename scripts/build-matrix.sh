#!/usr/bin/env bash
# 릴리즈 feature 매트릭스를 모두 --release 로 빌드한다.
# C-free 조합(default/remote)은 전 플랫폼, C 의존(storage/tls)은 C 툴체인 환경에서만.
# 사용: scripts/build-matrix.sh [--cfree-only]
set -euo pipefail

CFREE=("" "remote")
CDEP=("storage" "tls" "storage remote" "storage tls remote")

run() {
  local feats="$1"
  if [ -z "$feats" ]; then
    echo "==> build --release (default)"
    cargo build --release
  else
    echo "==> build --release --features \"$feats\""
    cargo build --release --features "$feats"
  fi
}

for f in "${CFREE[@]}"; do run "$f"; done

if [ "${1:-}" = "--cfree-only" ]; then
  echo "OK (C-free only)"
  exit 0
fi

for f in "${CDEP[@]}"; do run "$f"; done
echo "OK (full matrix)"
