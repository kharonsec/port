#!/bin/bash
set -e

echo "Building port..."
cargo build --release

echo "Installing port to /usr/local/bin/port..."
sudo cp target/release/port_cli /usr/local/bin/port

echo "Done!"
