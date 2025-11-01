#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(cd "${SCRIPT_DIR}/.." && pwd)

DEFAULT_INPUT="${PROJECT_ROOT}/test-data/scan/BLP1_tt1_c0_ab8_at0_m0_512x128/CenterPanel01.blp"
DEFAULT_INPUT="/Users/nazarpunk/Downloads/_blp/bb.blp"
DEFAULT_INPUT="/Users/nazarpunk/Downloads/_blp/logo.png"

cd "${PROJECT_ROOT}"

ARGS=("$@")
if [[ ${#ARGS[@]} -eq 0 ]]; then
    ARGS=("${DEFAULT_INPUT}")
fi

cargo run --release --bin blp-ui --features "cli ui" -- "${ARGS[@]}"
#cargo run --release --bin blp-ui --features "cli ui"
