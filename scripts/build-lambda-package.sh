#!/usr/bin/env bash
set -euo pipefail

# Functions for better logging
log() { echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" >&2; }
error() { log "ERROR: $*"; exit 1; }
debug() { log "DEBUG: $*"; }

# Set paths
INSTALL_DIR="${UV_INSTALL_DIR:-$HOME/.uv}"
LAMBDA_DIR="clients/lambda"
PACKAGE_DIR="$INSTALL_DIR/prisms/system/deploy/lambda"

# Ensure we're in the project root
[ -f "Cargo.toml" ] || error "Must be run from project root"

log "Building UV Lambda deployment package..."

# Check if Lambda client directory exists
[ -d "$LAMBDA_DIR" ] || error "Lambda client directory not found: $LAMBDA_DIR"

# Navigate to Lambda client directory
cd "$LAMBDA_DIR" || error "Failed to change to Lambda directory"

# Determine build target and command
if [[ "$OSTYPE" == "darwin"* ]]; then
    TARGET="aarch64-unknown-linux-gnu"
    
    # Check if target is installed
    if ! rustup target list --installed | grep -q "$TARGET"; then
        log "Installing target $TARGET..."
        rustup target add "$TARGET" || error "Failed to install target $TARGET"
    fi
    
    # Check if cross is installed
    if ! command -v cross &> /dev/null; then
        log "Installing cross for cross-compilation..."
        cargo install cross || error "Failed to install cross"
    fi
    
    BUILD_CMD="cross build --release --target $TARGET"
else
    TARGET="aarch64-unknown-linux-gnu"
    
    # Check if target is installed
    if ! rustup target list --installed | grep -q "$TARGET"; then
        log "Installing target $TARGET..."
        rustup target add "$TARGET" || error "Failed to install target $TARGET"
    fi
    
    BUILD_CMD="cargo build --release --target $TARGET"
fi

log "Running: $BUILD_CMD"
eval "$BUILD_CMD" || error "Failed to build Lambda client"

# Create package directory
log "Creating package directory: $PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR" || error "Failed to create package directory"

# Copy binary and rename to 'bootstrap' (required by Lambda)
BINARY_PATH="target/$TARGET/release/uv-lambda"
[ -f "$BINARY_PATH" ] || error "Lambda binary not found at $BINARY_PATH"

cp "$BINARY_PATH" ./bootstrap || error "Failed to copy Lambda binary"

# Create ZIP package
log "Creating Lambda deployment package..."
if command -v zip &> /dev/null; then
    zip -j "$PACKAGE_DIR/package.zip" ./bootstrap || error "Failed to create Lambda package"
    
    # Get package size for logging
    PACKAGE_SIZE=$(wc -c < "$PACKAGE_DIR/package.zip" | tr -d ' ')
    log "Lambda package created successfully: $PACKAGE_DIR/package.zip ($PACKAGE_SIZE bytes)"
else
    error "zip command not found. Please install zip utility."
fi

# Clean up temporary files
rm -f ./bootstrap

# Return to project root
cd - > /dev/null

log "Lambda package build completed successfully!"
log "Package location: $PACKAGE_DIR/package.zip"
log ""
log "To deploy this package using the deploy prism:"
log "  uv system:deploy.lambda '{\"functionName\": \"my-uv-function\"}'"
