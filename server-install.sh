#!/bin/bash

if [ "$EUID" -ne 0 ]
  then echo "Please run as root"
  exit
fi

if ! command -v systemctl &> /dev/null
then
    echo "Systemd is not installed. Please use docker instead."
    exit
fi

SYSTEMD_URL="https://raw.githubusercontent.com/filipton/local-forwarder/master/systemd/lf-server.service"
SYSTEMD_PATH="/etc/systemd/system/lf-server.service"

if [ -f "$SYSTEMD_PATH" ]; then
    echo "'lf-server' service already exists. Please remove it first."
    exit
fi

echo "Downloading systemd service file..."
curl -s -o "$SYSTEMD_PATH" "$SYSTEMD_URL"

echo "Reloading systemd daemon..."
systemctl daemon-reload

read -p "Do you want to enable auto-start of 'lf-server' service? [y/N] " -n 1 -r
if [[ $REPLY =~ ^[Yy]$ ]]
then
    echo "Enabling auto-start of 'lf-server' service..."
    systemctl enable lf-server.service
fi

echo "Starting 'lf-server' service..."
systemctl start lf-server.service
