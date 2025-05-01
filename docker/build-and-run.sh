#!/usr/bin/env bash
set -euo pipefail

# Default variables
IMAGE_NAME="ultraviolet-server"
CONTAINER_NAME="uv-server"
PORT=3000

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Help text
show_help() {
  echo "Usage: $0 [OPTIONS]"
  echo ""
  echo "Build and run the Ultraviolet server in Docker"
  echo ""
  echo "Options:"
  echo "  -h, --help          Show this help message"
  echo "  -p, --port PORT     Port to expose (default: 3000)"
  echo "  -n, --name NAME     Container name (default: uv-server)"
  echo "  -i, --image NAME    Image name (default: ultraviolet-server)"
  echo "  -r, --rebuild       Force rebuild of the image"
  echo "  -d, --detach        Run container in detached mode"
  echo ""
  exit 0
}

# Parse arguments
REBUILD=false
DETACH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      show_help
      ;;
    -p|--port)
      PORT="$2"
      shift 2
      ;;
    -n|--name)
      CONTAINER_NAME="$2"
      shift 2
      ;;
    -i|--image)
      IMAGE_NAME="$2"
      shift 2
      ;;
    -r|--rebuild)
      REBUILD=true
      shift
      ;;
    -d|--detach)
      DETACH="--detach"
      shift
      ;;
    *)
      echo "Unknown option: $1"
      show_help
      ;;
  esac
done

# Change to project root directory
cd "$PROJECT_ROOT"

# Check if image exists or rebuild flag is set
if [[ "$(docker images -q "$IMAGE_NAME" 2> /dev/null)" == "" ]] || [[ "$REBUILD" == true ]]; then
  echo "Building Docker image $IMAGE_NAME..."
  docker build -t "$IMAGE_NAME" .
else
  echo "Using existing Docker image $IMAGE_NAME. Use --rebuild to force rebuild."
fi

# Stop and remove existing container if it exists
if [[ "$(docker ps -a -q -f name="$CONTAINER_NAME" 2> /dev/null)" != "" ]]; then
  echo "Stopping and removing existing container $CONTAINER_NAME..."
  docker stop "$CONTAINER_NAME" 2> /dev/null || true
  docker rm "$CONTAINER_NAME" 2> /dev/null || true
fi

# Run the container
echo "Starting Ultraviolet server on port $PORT..."
docker run $DETACH --name "$CONTAINER_NAME" \
  -p "$PORT:3000" \
  "$IMAGE_NAME"

if [[ -z "$DETACH" ]]; then
  echo "Ultraviolet server running in foreground. Press Ctrl+C to stop."
else
  echo "Ultraviolet server running in background. Run 'docker stop $CONTAINER_NAME' to stop."
  echo "Access the server at http://localhost:$PORT/"
fi
