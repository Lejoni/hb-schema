#!/usr/bin/env bash
set -euo pipefail

# Determine script directory (root of the repository)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_CONFIG="$SCRIPT_DIR/config.toml"
RELEASE_BIN="$SCRIPT_DIR/target/release/hb-schema"

# Colors for terminal output
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BOLD}${BLUE}=== HB-Schema Installation ===${NC}\n"

SKIP_BUILD=0
BIN_DIR=""

# Parse optional arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-build)
            SKIP_BUILD=1
            shift
            ;;
        --bin-dir)
            BIN_DIR="$2"
            shift 2
            ;;
        --bin-dir=*)
            BIN_DIR="${1#*=}"
            shift
            ;;
        -h|--help)
            echo "Usage: ./install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --no-build          Skip building the binary with cargo"
            echo "  --bin-dir=<path>    Custom binary installation directory (default: ~/.local/bin)"
            echo "  -h, --help          Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option:${NC} $1"
            exit 1
            ;;
    esac
done

# Step 1: Build release binary if requested
if [[ $SKIP_BUILD -eq 0 ]]; then
    echo -e "${BOLD}[1/3] Building release binary...${NC}"
    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
    echo -e "${GREEN}✓ Build completed.${NC}\n"
else
    echo -e "${BOLD}[1/3] Skipping build (--no-build specified).${NC}\n"
fi

if [[ ! -f "$RELEASE_BIN" ]]; then
    echo -e "${RED}Error: Release binary not found at '$RELEASE_BIN'.${NC}"
    echo "Please build the project first with 'cargo build --release' or run without --no-build."
    exit 1
fi

# Step 2: Determine binary destination and install executable
if [[ -z "$BIN_DIR" ]]; then
    if [[ -n "${XDG_BIN_HOME:-}" ]]; then
        BIN_DIR="$XDG_BIN_HOME"
    else
        BIN_DIR="$HOME/.local/bin"
    fi
fi

echo -e "${BOLD}[2/3] Installing executable to '${BIN_DIR}'...${NC}"
mkdir -p "$BIN_DIR"
cp "$RELEASE_BIN" "$BIN_DIR/hb-schema"
chmod +x "$BIN_DIR/hb-schema"
echo -e "${GREEN}✓ Installed executable:${NC} $BIN_DIR/hb-schema\n"

# Step 3: Handle configuration file
echo -e "${BOLD}[3/3] Checking configuration file...${NC}"

if [[ ! -f "$PROJECT_CONFIG" ]]; then
    echo -e "${RED}Error: Project config file not found at '$PROJECT_CONFIG'.${NC}"
    exit 1
fi

# Candidate search paths in priority order
CANDIDATE_PATHS=()

if [[ -n "${XDG_CONFIG_HOME:-}" ]]; then
    CANDIDATE_PATHS+=(
        "$XDG_CONFIG_HOME/hb-schema/config.toml"
        "$XDG_CONFIG_HOME/hb-schema/hb-schema.toml"
        "$XDG_CONFIG_HOME/hb-schema.toml"
    )
fi

DOT_CONFIG="$HOME/.config"
if [[ "${XDG_CONFIG_HOME:-}" != "$DOT_CONFIG" ]]; then
    CANDIDATE_PATHS+=(
        "$DOT_CONFIG/hb-schema/config.toml"
        "$DOT_CONFIG/hb-schema/hb-schema.toml"
        "$DOT_CONFIG/hb-schema.toml"
    )
fi

EXISTING_CONFIG=""
for candidate in "${CANDIDATE_PATHS[@]}"; do
    if [[ -f "$candidate" ]]; then
        EXISTING_CONFIG="$candidate"
        break
    fi
done

if [[ -n "$EXISTING_CONFIG" ]]; then
    echo -e "Found existing configuration file at: ${YELLOW}$EXISTING_CONFIG${NC}"
    if cmp -s "$PROJECT_CONFIG" "$EXISTING_CONFIG"; then
        echo -e "${GREEN}✓ Existing config is already identical to the project configuration. No update needed.${NC}"
    else
        TIMESTAMP="$(date +%Y%m%d%H%M%S)"
        BACKUP_FILE="${EXISTING_CONFIG}.bak.${TIMESTAMP}"
        echo -e "Config differs. Creating backup at: ${YELLOW}$BACKUP_FILE${NC}"
        cp "$EXISTING_CONFIG" "$BACKUP_FILE"
        echo -e "Updating configuration at: ${YELLOW}$EXISTING_CONFIG${NC}"
        cp "$PROJECT_CONFIG" "$EXISTING_CONFIG"
        echo -e "${GREEN}✓ Config successfully updated (backup created).${NC}"
    fi
else
    if [[ -n "${XDG_CONFIG_HOME:-}" ]]; then
        TARGET_CONFIG="$XDG_CONFIG_HOME/hb-schema/config.toml"
    else
        TARGET_CONFIG="$HOME/.config/hb-schema/config.toml"
    fi

    echo -e "No existing config found in default search paths."
    echo -e "Installing new configuration to: ${YELLOW}$TARGET_CONFIG${NC}"
    mkdir -p "$(dirname "$TARGET_CONFIG")"
    cp "$PROJECT_CONFIG" "$TARGET_CONFIG"
    echo -e "${GREEN}✓ Config file installed successfully.${NC}"
fi

echo -e "\n${BOLD}${GREEN}=== Installation Complete ===${NC}"

# Check if BIN_DIR is in PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo -e "\n${YELLOW}Note:${NC} '${BIN_DIR}' is not in your current PATH."
    echo -e "You may want to add it to your ~/.bashrc or ~/.zshrc:"
    echo -e "    ${BOLD}export PATH=\"\$PATH:$BIN_DIR\"${NC}"
fi

echo -e "\nYou can now run '${BOLD}hb-schema${NC}' from your terminal."
