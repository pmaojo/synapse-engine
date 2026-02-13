#!/bin/bash
# Install systemd service for Synapse gRPC server
# Requires sudo

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_NAME="synapse-grpc"
SERVICE_FILE="$SCRIPT_DIR/synapse-grpc.service"
SYSTEMD_DIR="/etc/systemd/system"

if [ ! -f "$SERVICE_FILE" ]; then
    echo "Error: Service file $SERVICE_FILE not found."
    exit 1
fi

echo "Installing Synapse gRPC systemd service..."
sudo cp "$SERVICE_FILE" "$SYSTEMD_DIR/"
sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"
sudo systemctl start "$SERVICE_NAME"

echo "✅ Service installed and started."
echo "Check status:  sudo systemctl status $SERVICE_NAME"
echo "View logs:     sudo journalctl -u $SERVICE_NAME -f"