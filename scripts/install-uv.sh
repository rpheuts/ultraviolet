#!/usr/bin/env bash
set -euo pipefail

# Functions for better error handling and logging
log() { echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" >&2; }
error() { log "ERROR: $*"; exit 1; }
debug() { log "DEBUG: $*" >&2; }

# Install prism function
install_prism() {
    local prism_dir="$1"
    local install_dir="$2"
    local namespace_override="$3"
    local name_override="$4"
    
    # Skip CLI and core packages
    local name=$(basename "$prism_dir")
    case "$name" in
        "uv-cli"|"uv-core")
            return
            ;;
    esac
    
    # Validate prism
    [ -d "$prism_dir" ] || error "Invalid prism directory: $prism_dir"
    [ -f "$prism_dir/spectrum.json" ] || error "No spectrum.json found in $prism_dir"
    
    # Parse prism info
    local prism_name
    local lib_name
    
    if [ -n "$name_override" ]; then
        prism_name="$name_override"
        lib_name="uv_prism_${namespace_override}_${name_override}"
    else
        prism_name=${name#uv-prism-}
        lib_name=$(echo "$name" | tr '-' '_')
    fi
    
    # Get namespace from spectrum.json or override
    local namespace
    if [ -n "$namespace_override" ]; then
        namespace="$namespace_override"
    else
        # For old-style prisms, extract namespace from the spectrum.json
        namespace=$(jq -r '.namespace // "example"' "$prism_dir/spectrum.json")
        
        # For old-style prisms with "uv-prism-" prefix, the name is the part after the prefix
        if [[ "$name" == uv-prism-* ]]; then
            prism_name=${name#uv-prism-}
        fi
    fi
    
    log "Installing $prism_name to $namespace namespace"
    
    # Create prism directory
    local prism_install_dir="$install_dir/prisms/$namespace/$prism_name"
    mkdir -p "$prism_install_dir"
    
    # Copy files
    if [ -f "target/release/lib${lib_name}.dylib" ]; then
        cp "target/release/lib${lib_name}.dylib" "$prism_install_dir/module.dylib" || \
            error "Failed to copy prism binary for $prism_name"
    elif [ -f "target/release/lib${lib_name}.so" ]; then
        cp "target/release/lib${lib_name}.so" "$prism_install_dir/module.so" || \
            error "Failed to copy prism binary for $prism_name"
    elif [ -f "target/release/lib${lib_name}.dll" ]; then
        cp "target/release/lib${lib_name}.dll" "$prism_install_dir/module.dll" || \
            error "Failed to copy prism binary for $prism_name"
    else
        error "Could not find library for $prism_name (lib${lib_name})"
    fi
    
    cp "$prism_dir/spectrum.json" "$prism_install_dir/spectrum.json" || \
        error "Failed to copy spectrum.json for $prism_name"
    
    log "Successfully installed $prism_name"
}

# Main installation logic
main() {
    local install_dir="${UV_INSTALL_DIR:-$HOME/.uv}"
    
    # Ensure we're in the project root
    [ -f "Cargo.toml" ] || error "Must be run from project root"
    
    # Install CLI
    log "Installing UV CLI..."
    mkdir -p "$install_dir/bin"
    cp "target/release/cli" "$install_dir/bin/uv" || \
        error "Failed to install CLI"
    
    # Install prisms from old structure
    log "Installing prisms..."
    for prism in uv-prism-*/; do
        [ -d "$prism" ] || continue
        install_prism "$prism" "$install_dir" "" ""
    done
    
    # Install prisms from new structure
    for namespace_dir in prisms/*/; do
        [ -d "$namespace_dir" ] || continue
        
        namespace=$(basename "$namespace_dir")
        
        for prism_dir in "$namespace_dir"/*/; do
            [ -d "$prism_dir" ] || continue
            
            name=$(basename "$prism_dir")
            
            install_prism "$prism_dir" "$install_dir" "$namespace" "$name"
        done
    done

    # Install knowledge
    mkdir -p "$install_dir/knowledge"
    cp "ai/knowledge"/* "$install_dir/knowledge" || \
        error "Failed to install knowledge"
    
    log "Installation complete!"
}

main "$@"
