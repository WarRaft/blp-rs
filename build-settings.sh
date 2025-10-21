#!/usr/bin/env bash
# Общие переменные и хелперы для build-скриптов

set -euo pipefail

# ====== SETTINGS ======
export DIST_DIR="${DIST_DIR:-bin}"
export BUILD_VARIANTS="${BUILD_VARIANTS:-$'blp-cli:cli:binary\nblp-ui:cli,ui:app'}"
export APP_ID_BUNDLE="${APP_ID_BUNDLE:-com.blp}"
export UPX="${UPX:-0}"
# ======================

# Size-friendly флаги ТОЛЬКО на уровне rustc (без LTO и без embed-bitcode)
# LTO включим в Cargo.toml в профиле release.
_prev="${RUSTFLAGS:-}"
# вычистим возможные следы embed-bitcode из окружения
_prev="${_prev//-C embed-bitcode=no/}"
_prev="${_prev//-C embed-bitcode=yes/}"
export RUSTFLAGS="${_prev} -C codegen-units=1 -C opt-level=z -C strip=symbols -C panic=abort"
# =========================================

mkdir -p "$DIST_DIR"

need() { command -v "$1" &>/dev/null || { echo "❌ Требуется '$1'"; exit 1; }; }

find_llvm_strip() {
  local host sysroot p
  host="$(rustc -vV | sed -n 's/^host: //p')"
  sysroot="$(rustc --print sysroot)"
  for p in \
    "$sysroot/lib/rustlib/$host/bin/llvm-strip" \
    "$sysroot/lib/rustlib/bin/llvm-strip" \
    "/opt/homebrew/opt/llvm/bin/llvm-strip" \
    "/usr/local/opt/llvm/bin/llvm-strip" \
    "$(xcrun --find llvm-strip 2>/dev/null || true)"; do
    [[ -n "${p:-}" && -x "$p" ]] && { echo "$p"; return 0; }
  done
  return 1
}

LLVM_STRIP="$(find_llvm_strip || true)"

strip_safe() {
  # $1=path  $2=kind: macos|linux|windows
  local f="$1" kind="${2:-}"
  [[ -f "$f" ]] || return 0
  case "$kind" in
    macos) /usr/bin/strip -x "$f" || true ;;
    linux|windows)
      if [[ -n "$LLVM_STRIP" ]]; then "$LLVM_STRIP" -s "$f" || true
      else echo "⚠️  Пропускаю strip для $f (нет llvm-strip)"; fi
      ;;
    *) [[ -n "$LLVM_STRIP" ]] && "$LLVM_STRIP" -s "$f" || true ;;
  esac
}

maybe_upx() {
  if [[ "$UPX" = "1" ]] && command -v upx &>/dev/null; then
    upx --best --lzma "$1" || true
  fi
}
