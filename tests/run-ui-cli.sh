#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(cd "${SCRIPT_DIR}/.." && pwd)

cd "${PROJECT_ROOT}"

# если аргументов нет — показываем help вместо запуска GUI
if [[ $# -eq 0 ]]; then
  set -- --help
fi

cargo run --release \
  --bin blp-ui \
  --features "cli ui" \
  -- "$@"
