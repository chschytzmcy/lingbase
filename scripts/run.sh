#!/bin/bash
# Lingbase startup script - auto-detect architecture and load correct libraries

set -e

# POSIX compatible way to get script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# Project root directory (scripts/ is in project root)
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Detect system architecture
detect_arch() {
    local arch=$(uname -m)
    case "$arch" in
        x86_64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            echo "unsupported" >&2
            exit 1
            ;;
    esac
}

# Detect CUDA availability
detect_cuda() {
    if command -v nvidia-smi > /dev/null 2>&1; then
        if nvidia-smi > /dev/null 2>&1; then
            echo "cuda"
            return
        fi
    fi
    echo "cpu"
}

# Main library directory
LIB_DIR="$PROJECT_ROOT/lib"

# Auto-detect architecture
ARCH=$(detect_arch)
echo "[lingbase] Detected architecture: $ARCH"

# Auto-detect backend
BACKEND=$(detect_cuda)
echo "[lingbase] Detected backend: $BACKEND"

# Set library path based on architecture
LIB_ARCH_DIR="$LIB_DIR/$ARCH"
if [ ! -d "$LIB_ARCH_DIR" ]; then
    echo "[lingbase] Error: Library directory not found: $LIB_ARCH_DIR" >&2
    exit 1
fi

# For CUDA, also add CUDA library path
if [ "$BACKEND" = "cuda" ]; then
    LIB_ARCH_DIR="$LIB_ARCH_DIR:$LIB_DIR/cuda"
    echo "[lingbase] Loading CUDA libraries"
fi

# Set library path
export LD_LIBRARY_PATH="${LIB_ARCH_DIR}:${LD_LIBRARY_PATH}"

# Verify libraries exist
if [ ! -f "$LIB_ARCH_DIR/libllama.so" ]; then
    echo "[lingbase] Error: libllama.so not found in $LIB_ARCH_DIR" >&2
    exit 1
fi

echo "[lingbase] LD_LIBRARY_PATH=$LD_LIBRARY_PATH"

# Run the binary
exec ./target/release/lingbase "$@"