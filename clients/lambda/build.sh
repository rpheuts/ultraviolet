#!/bin/bash
set -e

echo "Building UV Lambda function..."

# Check if we're on macOS or Linux and set the appropriate target
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Building for macOS (cross-compiling to Linux ARM64)..."
    TARGET="aarch64-unknown-linux-gnu"
    
    # Check if the target is installed
    if ! rustup target list --installed | grep -q "$TARGET"; then
        echo "Installing target $TARGET..."
        rustup target add $TARGET
    fi
    
    # Install cross if not available (for cross-compilation)
    if ! command -v cross &> /dev/null; then
        echo "Installing cross for cross-compilation..."
        cargo install cross
        BUILD_CMD="cross"
    else
        BUILD_CMD="cross"
    fi
else
    echo "Building on Linux..."
    TARGET="aarch64-unknown-linux-gnu"
    BUILD_CMD="cargo"
fi

echo "Building with $BUILD_CMD for target $TARGET..."
$BUILD_CMD build --release --target $TARGET

echo "Creating Lambda deployment package..."

# Copy the binary and rename it to 'bootstrap' (required by Lambda custom runtime)
cp target/$TARGET/release/uv-lambda ./bootstrap

# Create deployment package
if command -v zip &> /dev/null; then
    zip -j uv-lambda.zip ./bootstrap
    echo "Created uv-lambda.zip"
else
    echo "Warning: zip command not found. You'll need to manually package the 'bootstrap' binary."
fi

# Clean up
rm -f ./bootstrap

echo "Build complete!"
echo ""
echo "To deploy with SAM CLI:"
echo "  sam deploy --guided"
echo ""
echo "Or to deploy with existing configuration:"
echo "  sam deploy"
echo ""
echo "Make sure you have AWS credentials configured and SAM CLI installed."
