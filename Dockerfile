# Base image: Rust for building
FROM rust:1.86-slim as builder

# Install build dependencies 
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    jq && \
    rm -rf /var/lib/apt/lists/*

# Copy source code
WORKDIR /build
COPY . .

# Build the project
RUN cargo build --release

# Run the installation script
RUN mkdir -p /build/.uv && \
    UV_INSTALL_DIR=/build/.uv ./scripts/install-uv.sh

# Base runtime image: Debian slim for minimal size
FROM debian:bookworm-slim as base-runtime

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    openssl \
    curl \
    ca-certificates \
    supervisor && \
    rm -rf /var/lib/apt/lists/*

# Create UV user to avoid running as root
RUN useradd -ms /bin/bash uvuser

# Set up UV directory structure
WORKDIR /home/uvuser

# Copy the entire .uv directory from builder stage
COPY --from=builder /build/.uv /home/uvuser/.uv

# Set proper ownership
RUN chown -R uvuser:uvuser /home/uvuser/.uv

# Configure supervisor for auto-restart
COPY docker/supervisord.conf /etc/supervisor/conf.d/uv.conf

# Set HOME environment variable explicitly
ENV HOME="/home/uvuser"

# Expose the server port
EXPOSE 3000

# Run supervisor as the entry point
ENTRYPOINT ["/usr/bin/supervisord", "-c", "/etc/supervisor/supervisord.conf"]

# Tailscale-enabled runtime image
FROM base-runtime as tailscale-runtime

# Install Tailscale
RUN curl -fsSL https://tailscale.com/install.sh | sh

# Add Tailscale entrypoint wrapper
COPY docker/tailscale-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Override the entrypoint
ENTRYPOINT ["/entrypoint.sh"]
