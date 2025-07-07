#!/bin/bash

# Build the project in release mode
cargo build --release

# Copy the binary to /usr/local/bin
sudo cp target/release/claude-launcher /usr/local/bin/

# Make it executable
sudo chmod +x /usr/local/bin/claude-launcher

echo "claude-launcher has been installed to /usr/local/bin/"
echo "You can now use it from anywhere by running: claude-launcher \"your task\""