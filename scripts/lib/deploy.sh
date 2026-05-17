#!/bin/bash
# Lingbase deploy library - internal functions

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[lingbase]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[lingbase]${NC} WARN: $*"; }
log_error() { echo -e "${RED}[lingbase]${NC} ERROR: $*" >&2; }
log_debug() { [[ "${DEBUG:-}" == "1" ]] && echo -e "${BLUE}[lingbase]${NC} DEBUG: $*" || true; }

# Get project root
get_project_root() {
    local caller_dir="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
    if [[ -f "$caller_dir/lingbase" ]] && [[ ! -d "$caller_dir/scripts" ]]; then
        echo "$caller_dir"
    else
        echo "$(cd "$caller_dir/.." && pwd)"
    fi
}

# Detect local architecture
detect_local_arch() {
    local arch=$(uname -m)
    case "$arch" in
        x86_64|amd64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) echo "unknown" ;;
    esac
}

# List available servers from deploy.toml
list_servers() {
    local config_file="${1:-config/deploy.toml}"
    if [[ ! -f "$config_file" ]]; then
        log_error "Config file not found: $config_file"
        return 1
    fi

    echo "Available servers:"
    echo ""
    grep '^\[remote\.' "$config_file" | sed 's/\[remote\.\(.*\)]/\1/' | while read server; do
        local desc=$(grep -A5 "\[remote.$server\]" "$config_file" | grep '^description' | sed 's/description = "\(.*\)"/\1/')
        echo "  - $server${desc:+ - $desc}"
    done
}

# Load remote config from deploy.toml
# Usage: load_remote_config "gpu-server"
load_remote_config() {
    local server_name="$1"
    local config_file="${2:-config/deploy.toml}"
    local section="[remote.$server_name]"
    local in_section=false
    local line

    REMOTE_HOST=""
    REMOTE_USER=""
    REMOTE_PORT="22"
    REMOTE_PASSWORD=""
    REMOTE_ARCH=""
    REMOTE_DIR=""
    REMOTE_LIB_DIR=""

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
            elif [[ "$line" =~ ^lib_dir[[:space:]]*= ]]; then
                REMOTE_LIB_DIR=$(echo "$line" | sed 's/lib_dir = "\(.*\)"/\1/')
            fi
        fi
    done < "$config_file"

    if [[ -z "$REMOTE_HOST" ]] || [[ -z "$REMOTE_USER" ]]; then
        log_error "Failed to load config for server: $server_name"
        return 1
    fi

    REMOTE_DIR="/home/${REMOTE_USER}/lingbase"
}

# Detect remote architecture via SSH
detect_remote_arch() {
    local user="$1"
    local host="$2"
    local port="${3:-22}"
    local password="$4"

    local remote_arch=$(sshpass -p "${password}" ssh -o StrictHostKeyChecking=no -p ${port} ${user}@${host} "uname -m" 2>/dev/null)

    case "$remote_arch" in
        x86_64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) echo "unknown" ;;
    esac
}

# Detect remote CUDA availability
detect_remote_cuda() {
    local user="$1"
    local host="$2"
    local port="${3:-22}"
    local password="$4"

    if sshpass -p "${password}" ssh -o StrictHostKeyChecking=no -p ${port} ${user}@${host} "nvidia-smi -L 2>/dev/null | head -1" 2>/dev/null | grep -q "GPU"; then
        echo "true"
    else
        echo "false"
    fi
}

# Deploy package to remote server
# Usage: deploy_to_remote "package_path" "user" "host" "port" "password" "remote_dir" "arch" "lib_dir"
deploy_to_remote() {
    local package="$1"
    local user="$2"
    local host="$3"
    local port="${4:-22}"
    local password="$5"
    local remote_dir="${6:-/home/${user}/lingbase}"
    local arch="${7:-}"
    local lib_dir="${8:-}"

    log_info "Upload package..."
    sshpass -p "${password}" ssh -o StrictHostKeyChecking=no -p ${port} ${user}@${host} "mkdir -p ${remote_dir}"
    sshpass -p "${password}" scp -o StrictHostKeyChecking=no -P ${port} "${package}" ${user}@${host}:${remote_dir}/

    log_info "Extract and configure..."
    sshpass -p "${password}" ssh -o StrictHostKeyChecking=no -p ${port} ${user}@${host} << 'EOF'
set -e
REMOTE_DIR="/home/${USER}/lingbase"
cd "$REMOTE_DIR"

PACKAGE_FILE=$(ls lingbase-*.tar.gz 2>/dev/null | head -1)
if [[ -z "$PACKAGE_FILE" ]]; then
    echo "ERROR: No package found"
    exit 1
fi

# Backup existing config if exists
EXTRACTED_DIR=$(tar -tzf "$PACKAGE_FILE" | head -1 | cut -d/ -f1)
CONFIG_BACKUP=""
if [[ -d "$REMOTE_DIR/config" ]]; then
    CONFIG_BACKUP="/tmp/config_backup_$(date +%Y%m%d_%H%M%S)"
    echo "[lingbase] Backup existing config to: $CONFIG_BACKUP"
    cp -r "$REMOTE_DIR/config" "$CONFIG_BACKUP"
fi

echo "Extracting: $PACKAGE_FILE"
tar -xzf "$PACKAGE_FILE"

# Restore existing config
if [[ -n "$CONFIG_BACKUP" ]]; then
    echo "[lingbase] Restore existing config..."
    rm -rf "${EXTRACTED_DIR}/config"
    mv "$CONFIG_BACKUP" "${EXTRACTED_DIR}/config"
fi

cd "$EXTRACTED_DIR"
chmod +x lingbase run.sh

if [[ -f "deploy/lingbase.service" ]]; then
    echo "[lingbase] Install systemd service..."
    sudo cp deploy/lingbase.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable lingbase
    echo "[lingbase] systemd service installed"
fi

echo "[lingbase] Extract complete: $(pwd)"
EOF
}

# Uninstall from remote server
# Usage: uninstall_from_remote "user" "host" "port" "password"
uninstall_from_remote() {
    local user="$1"
    local host="$2"
    local port="${3:-22}"
    local password="$4"
    local remote_dir="/home/${user}/lingbase"

    log_info "========== Uninstall lingbase =========="
    log_info "Target: ${user}@${host}:${remote_dir}"

    sshpass -p "${password}" ssh -o StrictHostKeyChecking=no -p ${port} ${user}@${host} << 'EOF'
set -e

if systemctl is-active --quiet lingbase 2>/dev/null; then
    echo "[lingbase] Stop systemd service..."
    sudo systemctl stop lingbase || true
    sudo systemctl disable lingbase || true
fi

sudo rm -f /etc/systemd/system/lingbase.service
sudo systemctl daemon-reload

if pgrep -f lingbase > /dev/null 2>&1; then
    echo "[lingbase] Stop running process..."
    pkill -f lingbase || true
    sleep 2
fi

if [[ -d "/home/etsme/lingbase" ]]; then
    echo "[lingbase] Delete installation directory..."
    rm -rf /home/etsme/lingbase
fi

echo "[lingbase] Uninstall complete"
EOF

    log_info "Uninstall complete!"
}

# Get version from Cargo.toml
get_version() {
    local cargo_toml="${1:-Cargo.toml}"
    grep '^version' "$cargo_toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
}