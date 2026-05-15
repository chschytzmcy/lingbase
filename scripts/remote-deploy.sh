#!/bin/bash
# Lingbase remote deploy script - build, package and deploy to remote server

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source deploy library
source "$SCRIPT_DIR/lib/deploy.sh"

CONFIG_FILE="config/deploy.toml"
VERSION=$(get_version)

# Usage
usage() {
    cat << EOF
Usage: $0 [server] [options]

Server:
  Available servers: $0 --list
  Default server: gpu-server

Options:
  -b, --build        Build only
  -p, --package      Package only (no deploy)
  -d, --deploy       Deploy only (use existing package)
  --uninstall        Uninstall from remote server
  --list             List available servers
  -h, --help         Show this help

Examples:
  $0                        # Build + package + deploy (default server)
  $0 gpu-server             # Deploy to gpu-server
  $0 gpu-server -b         # Build only
  $0 gpu-server -d         # Deploy only
  $0 --uninstall gpu-server # Uninstall from gpu-server
EOF
}

# Parse arguments
BUILD_ONLY=false
PACKAGE_ONLY=false
DEPLOY_ONLY=false
UNINSTALL=false
REMOTE_SERVER=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -b|--build) BUILD_ONLY=true; shift ;;
        -p|--package) PACKAGE_ONLY=true; shift ;;
        -d|--deploy) DEPLOY_ONLY=true; shift ;;
        --uninstall) UNINSTALL=true; shift ;;
        --list) list_servers "$CONFIG_FILE"; exit 0 ;;
        -h|--help) usage; exit 0 ;;
        -*) log_error "Unknown option: $1"; usage; exit 1 ;;
        *) REMOTE_SERVER="$1"; shift ;;
    esac
done

[[ -z "$REMOTE_SERVER" ]] && REMOTE_SERVER="gpu-server"

# Main
log_info "Lingbase v${VERSION} deploy tool"
log_info "Target server: $REMOTE_SERVER"

if $UNINSTALL; then
    load_remote_config "$REMOTE_SERVER" "$CONFIG_FILE"
    uninstall_from_remote "$REMOTE_USER" "$REMOTE_HOST" "$REMOTE_PORT" "$REMOTE_PASSWORD"
    exit 0
fi

# Load config and detect
load_remote_config "$REMOTE_SERVER" "$CONFIG_FILE"

if [[ -z "$REMOTE_ARCH" ]]; then
    REMOTE_ARCH=$(detect_remote_arch "$REMOTE_USER" "$REMOTE_HOST" "$REMOTE_PORT" "$REMOTE_PASSWORD")
fi

HAS_CUDA=$(detect_remote_cuda "$REMOTE_USER" "$REMOTE_HOST" "$REMOTE_PORT" "$REMOTE_PASSWORD")

log_info "Deploy target: ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_DIR}"
log_info "Arch: ${REMOTE_ARCH}, CUDA: ${HAS_CUDA}"

# Build
do_build() {
    local arch="${REMOTE_ARCH:-x86_64}"
    log_info "Build project (arch: $arch)..."

    if [[ "$arch" == "aarch64" ]]; then
        if ! command -v aarch64-linux-gnu-gcc > /dev/null 2>&1; then
            log_warn "aarch64-linux-gnu-gcc not found, installing..."
            sudo apt-get install -y gcc-aarch64-linux-gnu > /dev/null 2>&1 || true
        fi
        CARGO_TARGET=aarch64-unknown-linux-gnu cargo build --release --target aarch64-unknown-linux-gnu
    elif [[ "$HAS_CUDA" == "true" ]]; then
        cargo build --release --features cuda
    else
        cargo build --release
    fi

    [[ -f target/release/lingbase ]] || [[ -f target/aarch64-unknown-linux-gnu/release/lingbase ]] || {
        log_error "Build failed"
        exit 1
    }
    log_info "Build complete"
}

# Package
do_package() {
    local arch="${REMOTE_ARCH:-x86_64}"
    local package_name="lingbase-${VERSION}-${arch}"
    local package_tar="${package_name}"

    [[ "$HAS_CUDA" == "true" ]] && {
        package_name="lingbase-${VERSION}-${arch}-cuda"
        package_tar="${package_name}"
    }

    log_info "Package (arch: $arch, CUDA: $HAS_CUDA)..."

    rm -rf dist
    mkdir -p "dist/${package_name}/lib"
    mkdir -p "dist/${package_name}/config"

    local binary="target/release/lingbase"
    [[ "$arch" == "aarch64" ]] && binary="target/aarch64-unknown-linux-gnu/release/lingbase"
    [[ -f "$binary" ]] && cp "$binary" "dist/${package_name}/"

    if [[ "$HAS_CUDA" == "true" ]]; then
        # CUDA build: libraries in lib/cuda/
        mkdir -p "dist/${package_name}/lib/cuda"
        [[ -d "lib/cuda" ]] && cp -r lib/cuda/* "dist/${package_name}/lib/cuda/"
    else
        # CPU build: include arch-specific libraries in lib/x86_64/ or lib/aarch64/
        [[ -d "lib/${arch}" ]] && { mkdir -p "dist/${package_name}/lib/${arch}"; cp -r "lib/${arch}"/* "dist/${package_name}/lib/${arch}/"; }
    fi

    cp config/environment.toml "dist/${package_name}/config/"
    cp scripts/run.sh "dist/${package_name}/"
    [[ -d "deploy" ]] && cp -r deploy "dist/${package_name}/"

    tar -czf "dist/${package_tar}.tar.gz" -C dist "${package_name}"
    PACKAGE_FILE="dist/${package_tar}.tar.gz"
    log_info "Package complete: $PACKAGE_FILE ($(du -h ${PACKAGE_FILE} | cut -f1))"
}

# Deploy
do_deploy() {
    local arch="${REMOTE_ARCH:-x86_64}"
    local cuda_suffix=""
    [[ "$HAS_CUDA" == "true" ]] && cuda_suffix="-cuda"

    PACKAGE=$(ls -t dist/lingbase-${VERSION}-${arch}${cuda_suffix}.tar.gz 2>/dev/null | head -1)

    if [[ -z "$PACKAGE" ]]; then
        log_error "Package not found: dist/lingbase-${VERSION}-${arch}${cuda_suffix}.tar.gz"
        log_error "Run build first: $0 -b"
        exit 1
    fi

    log_info "Package: $(basename "$PACKAGE")"
    deploy_to_remote "$PACKAGE" "$REMOTE_USER" "$REMOTE_HOST" "$REMOTE_PORT" "$REMOTE_PASSWORD" "$REMOTE_DIR"
    log_info "Deploy complete!"
}

# Run
if $BUILD_ONLY; then
    do_build
elif $PACKAGE_ONLY; then
    do_build
    do_package
elif $DEPLOY_ONLY; then
    do_deploy
else
    do_build
    do_package
    do_deploy
fi

log_info "Done"