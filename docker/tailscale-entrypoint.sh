#!/bin/bash
set -e

# Start tailscaled in the background
tailscaled --state=/var/lib/tailscale/tailscaled.state --socket=/var/run/tailscale/tailscaled.sock &

# Wait for tailscaled to start
sleep 2

# Authenticate with Tailscale if TS_AUTHKEY is provided
if [ ! -z "${TS_AUTHKEY}" ]; then
  echo "Authenticating with Tailscale..."
  tailscale up --authkey="${TS_AUTHKEY}" --hostname="uv-server-$(hostname | cut -c1-8)" --accept-dns=false
  echo "Tailscale connected!"
else
  echo "No TS_AUTHKEY provided, Tailscale not connected"
fi

# Start supervisor as it would normally
exec /usr/bin/supervisord -c /etc/supervisor/supervisord.conf
