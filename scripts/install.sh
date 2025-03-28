#!/usr/bin/env bash
set -euo pipefail

# Functions for better error handling and logging
log() { echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" >&2; }
error() { log "ERROR: $*"; exit 1; }
debug() { log "DEBUG: $*" >&2; }

# Parse manifest function
parse_manifest() {
    local manifest="$1"
    local field="$2"
    local value=""
    
    # Use toml parsing if available
    if command -v toml2json >/dev/null; then
        value=$(toml2json < "$manifest" | jq -r ".module.$field // \"\"")
    else
        # Fallback to basic parsing with debug output
        value=$(awk -F'"' "/^[[:space:]]*$field[[:space:]]*=/ {print \$2}" "$manifest")
    fi
    
    # Default to 'blue' namespace if not specified
    if [ "$field" = "namespace" ] && [ -z "$value" ]; then
        echo "blue"
    else
        echo "$value"
    fi
}

# Validate module function
validate_module() {
    local module_dir="$1"
    local manifest="$module_dir/manifest.toml"
    
    [ -f "$manifest" ] || error "No manifest found in $module_dir"
    [ -d "$module_dir/src" ] || error "No source directory found in $module_dir"
    
    local name=$(basename "$module_dir")
    [[ "$name" =~ ^blue- ]] || error "Module directory must start with 'blue-'"
}

# Install module function
install_module() {
    local module_dir="$1"
    local install_dir="$2"
    local manifest="$module_dir/manifest.toml"
    
    # Skip CLI and core packages
    local name=$(basename "$module_dir")
    case "$name" in
        "blue-cli"|"blue-core"|"blue-render-cli"|"blue-render-core")
            return
            ;;
    esac
    
    # Validate module
    validate_module "$module_dir"
    
    # Parse module info
    local module_name=${name#blue-}
    local lib_name=$(echo "$name" | tr '-' '_')
    
    local namespace=$(parse_manifest "$manifest" "namespace")
    local tags=$(parse_manifest "$manifest" "tags")
    
    log "Installing $name to $namespace namespace (tags: $tags)"
    
    # Create module directory
    local module_install_dir="$install_dir/modules/$namespace/$module_name"
    mkdir -p "$module_install_dir"
    
    # Copy files
    cp "target/release/lib${lib_name}.dylib" "$module_install_dir/module.dylib" || \
        error "Failed to copy module binary for $name"
    cp "$manifest" "$module_install_dir/manifest.toml" || \
        error "Failed to copy manifest for $name"
    
    log "Successfully installed $name"
}

# Main installation logic
main() {
    local install_dir="${BLUE_INSTALL_DIR:-$HOME/.blue}"
    
    # Ensure we're in the project root
    [ -f "Cargo.toml" ] || error "Must be run from project root"
    
    # Install CLI
    log "Installing Blue CLI..."
    mkdir -p "$install_dir/bin"
    cp "target/release/blue-cli" "$install_dir/bin/blue" || \
        error "Failed to install CLI"

    cp "target/release/blue-module-runner" "$install_dir/bin/blue-module-runner" || \
        error "Failed to install Module Runner"
    
    # Install modules
    log "Installing modules..."
    for module in blue-*/; do
        [ -d "$module" ] || continue
        [[ "$module" != *"runner"* ]] || continue
        install_module "$module" "$install_dir"
    done

    mkdir -p "$install_dir/processes"
    
    log "Installation complete!"
}

main "$@"
