#!/bin/bash
# Common functions for Lingbase scripts

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[lingbase]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[lingbase]${NC} WARN: $*"
}

log_error() {
    echo -e "${RED}[lingbase]${NC} ERROR: $*" >&2
}

log_debug() {
    if [[ "${DEBUG:-}" == "1" ]]; then
        echo -e "${BLUE}[lingbase]${NC} DEBUG: $*"
    fi
}

# Get script directory (works for both regular and symlinked scripts)
get_script_dir() {
    local source="${BASH_SOURCE[0]}"
    while [[ -L "$source" ]]; do
        source="$(readlink "$source")"
    done
    echo "$(cd "$(dirname "$source")" && pwd)"
}

# Detect if running in packaged mode
# In packaged mode: run.sh is in package root with lingbase binary
# In development mode: scripts are in scripts/ subdirectory
is_packaged_mode() {
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
    # Packaged: binary exists in script dir, no scripts/ subdirectory
    [[ -f "$script_dir/lingbase" ]] && [[ ! -d "$script_dir/scripts" ]]
}

# Get project root directory
get_project_root() {
    local caller_dir
    caller_dir="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"

    if [[ -f "$caller_dir/lingbase" ]] && [[ ! -d "$caller_dir/scripts" ]]; then
        # Packaged mode: caller dir IS the project root
        echo "$caller_dir"
    else
        # Development mode: caller dir is scripts/, go up one level
        echo "$(cd "$caller_dir/.." && pwd)"
    fi
}

# Detect system architecture
detect_arch() {
    local arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            return 1
            ;;
    esac
}

# Detect CUDA availability (local)
detect_cuda() {
    if command -v nvidia-smi > /dev/null 2>&1; then
        if nvidia-smi > /dev/null 2>&1; then
            echo "cuda"
            return 0
        fi
    fi
    echo "cpu"
}

# Load remote config from deploy.toml
# Usage: load_remote_config "gpu-server-1"
load_remote_config() {
    local remote_server="$1"
    local config_file="config/deploy.toml"
    local section="[remote.$remote_server]"
    local in_section=false
    local line

    if [[ ! -f "$config_file" ]]; then
        log_error "配置文件未找到: $config_file"
        return 1
    fi

    while IFS= read -r line; do
        if [[ "$line" =~ \[remote\.[^]]+\] ]]; then
            if [[ "$line" == "$section" ]]; then
                in_section=true
            else
                in_section=false
            fi
        elif $in_section; then
            if [[ "$line" =~ ^host[[:space:]]*= ]]; then
                REMOTE_HOST=$(echo "$line" | sed 's/host = "\(.*\)"/\1/')
            elif [[ "$line" =~ ^user[[:space:]]*= ]]; then
                REMOTE_USER=$(echo "$line" | sed 's/user = "\(.*\)"/\1/')
            elif [[ "$line" =~ ^port[[:space:]]*= ]]; then
                REMOTE_PORT=$(echo "$line" | sed 's/port = \([0-9]*\)/\1/')
            elif [[ "$line" =~ ^password[[:space:]]*= ]]; then
                REMOTE_PASSWORD=$(echo "$line" | sed 's/password = "\(.*\)"/\1/')
            elif [[ "$line" =~ ^arch[[:space:]]*= ]]; then
                REMOTE_ARCH=$(echo "$line" | sed 's/arch = "\(.*\)"/\1/')
            fi
        fi
    done < "$config_file"

    if [[ -z "$REMOTE_HOST" ]] || [[ -z "$REMOTE_USER" ]]; then
        log_error "无法加载远程服务器配置: $remote_server"
        return 1
    fi

    REMOTE_DIR="/home/${REMOTE_USER}/lingbase"
}

# Load remote config and return arch (if specified in config)
load_remote_config_with_arch() {
    load_remote_config "$1"
    # REMOTE_ARCH will be set if arch is specified, otherwise empty
}

# Detect remote server architecture via SSH
detect_remote_arch() {
    local remote_user="$1"
    local remote_host="$2"
    local remote_port="${3:-22}"
    local remote_password="$4"

    local remote_arch=$(sshpass -p "${remote_password}" ssh -o StrictHostKeyChecking=no -p ${remote_port} ${remote_user}@${remote_host} "uname -m" 2>/dev/null)

    case "$remote_arch" in
        x86_64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            log_error "无法识别远程架构: $remote_arch"
            return 1
            ;;
    esac
}

# Detect if remote server has CUDA via SSH
detect_remote_cuda() {
    local remote_user="$1"
    local remote_host="$2"
    local remote_port="${3:-22}"
    local remote_password="$4"

    if sshpass -p "${remote_password}" ssh -o StrictHostKeyChecking=no -p ${remote_port} ${remote_user}@${remote_host} "command -v nvidia-smi > /dev/null 2>&1 && nvidia-smi > /dev/null 2>&1" 2>/dev/null; then
        echo "cuda"
        return 0
    else
        echo "cpu"
        return 1
    fi
}

# Validate required commands exist
require_commands() {
    local missing=()
    for cmd in "$@"; do
        if ! command -v "$cmd" > /dev/null 2>&1; then
            missing+=("$cmd")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required commands: ${missing[*]}"
        log_error "Please install them and try again"
        return 1
    fi
}

# Get version from Cargo.toml
get_version() {
    local cargo_toml="${1:-Cargo.toml}"
    grep '^version' "$cargo_toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
}