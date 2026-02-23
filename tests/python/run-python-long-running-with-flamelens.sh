#!/usr/bin/env bash
set -euo pipefail

PYTHON_VERSION="${1:-3.12}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PYTHON_SCRIPT="${SCRIPT_DIR}/long_running.py"
FLAMELENS_BIN="${ROOT_DIR}/target/release/flamelens"
PYTHON_BIN=""
PYTHON_PID=""

cleanup() {
    if [[ -n "${PYTHON_PID}" ]] && kill -0 "${PYTHON_PID}" 2>/dev/null; then
        kill "${PYTHON_PID}" 2>/dev/null || true
        wait "${PYTHON_PID}" 2>/dev/null || true
    fi
}

trap cleanup EXIT INT TERM

cd "${ROOT_DIR}"

echo "Building flamelens (release, all features)..."
cargo build --release --all-features

echo "Resolving Python binary via uv (Python ${PYTHON_VERSION})..."
PYTHON_BIN="$(uv run --python "${PYTHON_VERSION}" python -c 'import sys; print(sys.executable)')"
if [[ -z "${PYTHON_BIN}" || ! -x "${PYTHON_BIN}" ]]; then
    echo "Failed to resolve a runnable Python binary via uv for Python ${PYTHON_VERSION}" >&2
    exit 1
fi

echo "Python binary: ${PYTHON_BIN}"
echo "Starting Python workload..."
"${PYTHON_BIN}" "${PYTHON_SCRIPT}" >/dev/null 2>&1 &
PYTHON_PID=$!

echo "Python PID: ${PYTHON_PID}"
echo "Starting flamelens..."
sudo "${FLAMELENS_BIN}" --pid "${PYTHON_PID}"
