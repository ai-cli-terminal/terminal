#!/usr/bin/env bash
set -euo pipefail

root="${AI_TERMINAL_DEPS_ROOT:-$HOME/.local/opt/ai-terminal-deps/root}"
cache="${AI_TERMINAL_APT_CACHE:-$HOME/.cache/ai-terminal-apt}"

mkdir -p "$root" "$cache" "$HOME/.local/bin"
cd "$cache"

apt download nsis nsis-common proot libtalloc2 >/dev/null

for deb in nsis_*.deb nsis-common_*.deb proot_*.deb libtalloc2_*.deb; do
  dpkg-deb -x "$deb" "$root"
done

cat > "$HOME/.local/bin/makensis" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
root="${AI_TERMINAL_DEPS_ROOT:-$HOME/.local/opt/ai-terminal-deps/root}"
export LD_LIBRARY_PATH="$root/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
exec "$root/usr/bin/proot" -b "$root/usr/share/nsis:/usr/share/nsis" "$root/usr/bin/makensis" "$@"
EOF

cp "$HOME/.local/bin/makensis" "$HOME/.local/bin/makensis.exe"
chmod +x "$HOME/.local/bin/makensis" "$HOME/.local/bin/makensis.exe"

"$HOME/.local/bin/makensis" -VERSION
