.PHONY: build install clean test lint build-web image-base image-tailscale run-base run-tailscale

# Docker image names
DOCKER_BASE_IMAGE := ultraviolet-server
DOCKER_TAILSCALE_IMAGE := ultraviolet-server-tailscale
DOCKER_PORT := 3000

build:
	cargo build --release

install: build
	@scripts/install-uv.sh

build-web:
	@scripts/build-web.sh

install-web: build-web
	@echo "Web client installed to ~/.uv/assets/web"

# Build the base Docker image
image-base:
	@echo "Building base Docker image: $(DOCKER_BASE_IMAGE)"
	docker build --target base-runtime -t $(DOCKER_BASE_IMAGE):latest .

# Build the Tailscale-enabled Docker image
image-tailscale: 
	@echo "Building Tailscale-enabled Docker image: $(DOCKER_TAILSCALE_IMAGE)"
	docker build --target tailscale-runtime -t $(DOCKER_TAILSCALE_IMAGE):latest .

# Build all Docker images
image-all: image-base image-tailscale
	@echo "All Docker images built successfully"

# Run the base Docker image
run-base: image-base
	@echo "Running base Docker image on port $(DOCKER_PORT)"
	docker run --rm -p $(DOCKER_PORT):3000 --name uv-server $(DOCKER_BASE_IMAGE):latest

# Run the Tailscale-enabled Docker image
# Note: Requires TS_AUTHKEY environment variable to be set for Tailscale authentication
run-tailscale: #image-tailscale
	@echo "Running Tailscale-enabled Docker image on port $(DOCKER_PORT)"
	docker run --rm -p $(DOCKER_PORT):3000 --name uv-server-ts \
		-e HOME="/home/uvuser" \
		$(if $(TS_AUTHKEY),-e TS_AUTHKEY="$(TS_AUTHKEY)",) \
		--cap-add=NET_ADMIN \
		$(DOCKER_TAILSCALE_IMAGE):latest

test:
	cargo test --all

lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

clean:
	cargo clean
	rm -rf ~/.uv
	@echo "To clean Docker images, run: docker rmi $(DOCKER_BASE_IMAGE) $(DOCKER_TAILSCALE_IMAGE)"
