#!/usr/bin/env bash
set -euo pipefail

# Functions for better error handling and logging
log() { echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" >&2; }
error() { log "ERROR: $*"; exit 1; }

# Default installation directory
install_dir="${UV_INSTALL_DIR:-$HOME/.uv}"
assets_dir="$install_dir/assets/web"
web_client_dir="clients/ultraviolet-web"

# Ensure we're in the project root
[ -f "Cargo.toml" ] || error "Must be run from project root"
[ -d "$web_client_dir" ] || error "Web client directory not found: $web_client_dir"

# Create assets directory
log "Creating assets directory: $assets_dir"
mkdir -p "$assets_dir"

# Build the web client
log "Building web client..."
(
    cd "$web_client_dir" || error "Failed to change to web client directory"
    
    # Check if npm is installed
    command -v npm >/dev/null 2>&1 || error "npm is required but not installed"
    
    # Install dependencies if node_modules doesn't exist
    if [ ! -d "node_modules" ]; then
        log "Installing dependencies..."
        npm install || error "Failed to install dependencies"
    fi
    
    # Build the project
    log "Running build..."
    npm run build || error "Failed to build web client"
)

# Clean the assets directory
log "Cleaning assets directory..."
rm -rf "$assets_dir"/* || error "Failed to clean assets directory"

# Copy the build to the assets directory
log "Copying build to assets directory..."
cp -r "$web_client_dir/build/"* "$assets_dir/" || error "Failed to copy build files"

log "Web client successfully built and deployed to $assets_dir"
log "You can now access the web client at http://localhost:3000 when the UV service is running"
