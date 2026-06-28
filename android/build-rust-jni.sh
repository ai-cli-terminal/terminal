#!/usr/bin/env bash
set -euo pipefail

profile="debug"
android_api="26"
targets=(
  "aarch64-linux-android"
  "armv7-linux-androideabi"
  "i686-linux-android"
  "x86_64-linux-android"
)
install_rustup_targets=1

usage() {
  cat <<'EOF'
Usage: android/build-rust-jni.sh [--profile debug|release] [--android-api 26] [--target triple]... [--no-rustup-target-install]

Build Rust libai_terminal.so for Android and copy it into android/app/src/main/jniLibs/<abi>.

Set ANDROID_NDK_HOME to an Android NDK with a linux-x86_64 prebuilt toolchain, or set ANDROID_HOME/ANDROID_SDK_ROOT
to an SDK containing ndk/<version>.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      profile="${2:?missing profile}"
      shift 2
      ;;
    --android-api)
      android_api="${2:?missing Android API}"
      shift 2
      ;;
    --target)
      if [[ ${#targets[@]} -eq 4 && "${targets[0]}" == "aarch64-linux-android" ]]; then
        targets=()
      fi
      targets+=("${2:?missing target}")
      shift 2
      ;;
    --no-rustup-target-install)
      install_rustup_targets=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$profile" != "debug" && "$profile" != "release" ]]; then
  echo "profile must be debug or release" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

sdk_root="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
if [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
  ndk_root="$ANDROID_NDK_HOME"
else
  ndk_dir="$sdk_root/ndk"
  ndk_root="$(find "$ndk_dir" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -r | head -n 1 || true)"
fi
if [[ -z "${ndk_root:-}" || ! -d "$ndk_root" ]]; then
  echo "Android NDK not found. Set ANDROID_NDK_HOME or install an NDK under ANDROID_HOME/ndk." >&2
  exit 1
fi

toolchain_bin="$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin"
if [[ ! -d "$toolchain_bin" ]]; then
  echo "Linux NDK toolchain not found: $toolchain_bin" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found on PATH" >&2
  exit 1
fi

abi_for_target() {
  case "$1" in
    aarch64-linux-android) echo "arm64-v8a" ;;
    armv7-linux-androideabi) echo "armeabi-v7a" ;;
    i686-linux-android) echo "x86" ;;
    x86_64-linux-android) echo "x86_64" ;;
    *) return 1 ;;
  esac
}

linker_for_target() {
  case "$1" in
    aarch64-linux-android) echo "aarch64-linux-android${android_api}-clang" ;;
    armv7-linux-androideabi) echo "armv7a-linux-androideabi${android_api}-clang" ;;
    i686-linux-android) echo "i686-linux-android${android_api}-clang" ;;
    x86_64-linux-android) echo "x86_64-linux-android${android_api}-clang" ;;
    *) return 1 ;;
  esac
}

for target in "${targets[@]}"; do
  abi="$(abi_for_target "$target")" || {
    echo "unsupported Android Rust target: $target" >&2
    exit 2
  }
  linker="$toolchain_bin/$(linker_for_target "$target")"
  if [[ ! -x "$linker" ]]; then
    echo "NDK linker not found or not executable: $linker" >&2
    exit 1
  fi

  if [[ "$install_rustup_targets" -eq 1 ]] && command -v rustup >/dev/null 2>&1; then
    rustup target add "$target"
  fi

  env_name="CARGO_TARGET_$(echo "$target" | tr '[:lower:]-' '[:upper:]_')_LINKER"
  export "$env_name=$linker"

  cargo_args=(build --lib --target "$target")
  if [[ "$profile" == "release" ]]; then
    cargo_args+=(--release)
  fi
  (cd "$repo_root" && cargo "${cargo_args[@]}")

  profile_dir="$profile"
  source="$repo_root/target/$target/$profile_dir/libai_terminal.so"
  dest_dir="$script_dir/app/src/main/jniLibs/$abi"
  mkdir -p "$dest_dir"
  cp "$source" "$dest_dir/libai_terminal.so"
  echo "copied $source -> $dest_dir"
done
