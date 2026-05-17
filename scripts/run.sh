#!/bin/bash
# Lingbase startup script - auto-detect architecture and load correct libraries

set -e

# Project root detection - supports both project layout and package layout
SCRIPT_SOURCE="$0"
while [ -L "$SCRIPT_SOURCE" ]; do
    SCRIPT_SOURCE="$(readlink "$SCRIPT_SOURCE")"
done
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_SOURCE")" && pwd)"

# Check if lib/ is next to run.sh (package layout) or in parent (project layout)
# Package layout: run.sh and lib/ are siblings, lib/ contains arch subdirs
# Project layout: lib/ is in parent directory of scripts/, with arch subdirs
if [ -d "$SCRIPT_DIR/lib" ] && [ -d "$SCRIPT_DIR/lib/x86_64-cpu" -o -d "$SCRIPT_DIR/lib/aarch64" -o -d "$SCRIPT_DIR/lib/x86_64-cuda" ]; then
    # Package layout: run.sh and lib/ are siblings
    cd "$SCRIPT_DIR"
    PROJECT_ROOT="$SCRIPT_DIR"
elif [ -d "$SCRIPT_DIR/../lib" ] && [ -d "$SCRIPT_DIR/../lib/x86_64-cpu" -o -d "$SCRIPT_DIR/../lib/aarch64" -o -d "$SCRIPT_DIR/../lib/x86_64-cuda" ]; then
    # Project layout: lib/ is in parent directory of scripts/
    PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    cd "$PROJECT_ROOT"
else
    echo "[lingbase] Error: lib/ directory not found" >&2
    exit 1
fi

# Detect system architecture
detect_arch() {
    local arch="$(uname -m)"
    case "$arch" in
        x86_64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) echo "unsupported" >&2; exit 1 ;;
    esac
}

# Detect CUDA availability
detect_cuda() {
    if command -v nvidia-smi > /dev/null 2>&1 && nvidia-smi > /dev/null 2>&1; then
        echo "cuda"
    else
        echo "cpu"
    fi
}

ARCH="$(detect_arch)"
BACKEND="$(detect_cuda)"

echo "[lingbase] Detected architecture: $ARCH"
echo "[lingbase] Detected backend: $BACKEND"

LIB_DIR="$PROJECT_ROOT/lib"

# Determine library path based on architecture and backend
# Structure: lib/x86_64-cuda/ (x86 CUDA), lib/x86_64-cpu/ (x86 CPU), lib/aarch64/ (ARM CPU)
LIB_ARCH_DIR=""

if [ "$BACKEND" = "cuda" ]; then
    if [ -d "$LIB_DIR/x86_64-cuda" ]; then
        LIB_ARCH_DIR="$LIB_DIR/x86_64-cuda"
        echo "[lingbase] Loading CUDA libraries from: $LIB_DIR/x86_64-cuda"
    elif [ -f "$LIB_DIR/libggml-cuda.so" ]; then
        # Fallback: new packaging may have CUDA libs directly in lib/
        LIB_ARCH_DIR="$LIB_DIR"
        echo "[lingbase] Loading CUDA libraries from: $LIB_DIR (fallback)"
    else
        echo "[lingbase] Error: CUDA libraries not found" >&2
        exit 1
    fi
elif [ -d "$LIB_DIR/$ARCH" ]; then
    LIB_ARCH_DIR="$LIB_DIR/$ARCH"
    echo "[lingbase] Loading $ARCH libraries from: $LIB_DIR/$ARCH"
else
    echo "[lingbase] Error: Library directory not found" >&2
    echo "[lingbase] Expected: $LIB_DIR/x86_64-cuda, $LIB_DIR/x86_64-cpu, or $LIB_DIR/aarch64" >&2
    exit 1
fi

export LD_LIBRARY_PATH="${LIB_ARCH_DIR}:${LD_LIBRARY_PATH}"

# Verify libraries exist
if [ ! -f "$LIB_ARCH_DIR/libllama.so" ]; then
    echo "[lingbase] Error: libllama.so not found in $LIB_ARCH_DIR" >&2
    exit 1
fi

# Find and run the binary
if [ -x "$PROJECT_ROOT/lingbase" ]; then
    exec "$PROJECT_ROOT/lingbase" "$@"
elif [ -x "$PROJECT_ROOT/target/release/lingbase" ]; then
    exec "$PROJECT_ROOT/target/release/lingbase" "$@"
else
    echo "[lingbase] Error: lingbase binary not found" >&2
    exit 1
fi