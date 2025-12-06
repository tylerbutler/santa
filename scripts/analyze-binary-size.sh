#!/bin/bash
# analyze-binary-size.sh - Analyze Rust binary size impact of features
#
# Usage:
#   ./scripts/analyze-binary-size.sh           # Analyze santa binary with crate breakdown
#   ./scripts/analyze-binary-size.sh --quick   # Quick analysis without rebuilds
#   ./scripts/analyze-binary-size.sh --all     # Full analysis including function breakdown

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}\n"
}

print_section() {
    echo -e "\n${YELLOW}─── $1 ───${NC}\n"
}

get_binary_size() {
    local binary="$1"
    if [[ -f "$binary" ]]; then
        stat -c%s "$binary" 2>/dev/null || stat -f%z "$binary" 2>/dev/null
    else
        echo "0"
    fi
}

format_size() {
    local size=$1
    if [[ $size -ge 1048576 ]]; then
        printf "%.2f MB" "$(echo "scale=2; $size / 1048576" | bc)"
    elif [[ $size -ge 1024 ]]; then
        printf "%.2f KB" "$(echo "scale=2; $size / 1024" | bc)"
    else
        echo "$size B"
    fi
}

# Check for required tools
check_tools() {
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: cargo is required${NC}"
        exit 1
    fi

    if ! command -v cargo-bloat &> /dev/null; then
        echo -e "${YELLOW}cargo-bloat not installed. Installing...${NC}"
        cargo install cargo-bloat --quiet
    fi
}

# Build and get size for a specific configuration
build_config() {
    local description="$1"
    local cargo_args="$2"

    printf "  %-40s " "$description"

    # Build
    eval "cargo build --release $cargo_args" > /dev/null 2>&1

    local binary="target/release/santa"
    local size=$(get_binary_size "$binary")
    local formatted=$(format_size "$size")

    echo -e "${GREEN}$formatted${NC} ($size bytes)"
    echo "$size"
}

# Main analysis
main() {
    local mode="${1:-}"

    check_tools

    print_header "Santa Binary Size Analysis"

    # Current binary size
    print_section "Current Binary Size"

    if [[ ! -f "target/release/santa" ]]; then
        echo "Building release binary..."
        cargo build --release -p santa --quiet
    fi

    local current_size=$(get_binary_size "target/release/santa")
    echo -e "  Current santa binary: ${CYAN}$(format_size $current_size)${NC} ($current_size bytes)"

    # Per-crate breakdown (always show this)
    print_section "Size by Crate (top 25)"
    cargo bloat --release -p santa --crates -n 25

    # Profile comparison
    if [[ "$mode" != "--quick" ]]; then
        print_section "Profile Comparison"

        echo "Building with different profiles..."
        echo ""

        # Default release
        printf "  %-40s " "release (default)"
        cargo build --release -p santa --quiet 2>/dev/null
        local default_size=$(get_binary_size "target/release/santa")
        echo -e "${GREEN}$(format_size $default_size)${NC}"

        # With LTO
        printf "  %-40s " "release + lto=true"
        CARGO_PROFILE_RELEASE_LTO=true cargo build --release -p santa --quiet 2>/dev/null
        local lto_size=$(get_binary_size "target/release/santa")
        local lto_delta=$((default_size - lto_size))
        echo -e "${GREEN}$(format_size $lto_size)${NC} (${CYAN}-$(format_size $lto_delta)${NC})"

        # With strip
        printf "  %-40s " "release + strip=true"
        CARGO_PROFILE_RELEASE_STRIP=true cargo build --release -p santa --quiet 2>/dev/null
        local strip_size=$(get_binary_size "target/release/santa")
        local strip_delta=$((default_size - strip_size))
        echo -e "${GREEN}$(format_size $strip_size)${NC} (${CYAN}-$(format_size $strip_delta)${NC})"

        # With both LTO and strip
        printf "  %-40s " "release + lto + strip"
        CARGO_PROFILE_RELEASE_LTO=true CARGO_PROFILE_RELEASE_STRIP=true cargo build --release -p santa --quiet 2>/dev/null
        local both_size=$(get_binary_size "target/release/santa")
        local both_delta=$((default_size - both_size))
        echo -e "${GREEN}$(format_size $both_size)${NC} (${CYAN}-$(format_size $both_delta)${NC})"

        # Rebuild default for accurate state
        cargo build --release -p santa --quiet 2>/dev/null
    fi

    # Function breakdown (only with --all)
    if [[ "$mode" == "--all" ]]; then
        print_section "Largest Functions (top 30)"
        cargo bloat --release -p santa -n 30

        print_section "Largest Generic Functions"
        cargo bloat --release -p santa --filter ".*<.*>" -n 20
    fi

    # Dependency analysis
    print_section "Heavy Dependencies Analysis"
    echo "Crates contributing most to binary size:"
    echo ""
    cargo bloat --release -p santa --crates -n 10 --message-format csv 2>/dev/null | \
        tail -n +2 | \
        awk -F',' '{printf "  %-30s %s\n", $1, $2}'

    print_header "Recommendations"

    echo -e "To reduce binary size, add to ${CYAN}[profile.release]${NC} in workspace Cargo.toml:"
    echo ""
    echo -e "  ${GREEN}lto = true${NC}           # Link-time optimization"
    echo -e "  ${GREEN}codegen-units = 1${NC}    # Better optimization (slower build)"
    echo -e "  ${GREEN}strip = true${NC}         # Remove debug symbols"
    echo -e "  ${GREEN}panic = \"abort\"${NC}      # Smaller panic handling"
    echo ""
    echo -e "For more details, run: ${YELLOW}./scripts/analyze-binary-size.sh --all${NC}"
    echo ""
}

main "$@"
