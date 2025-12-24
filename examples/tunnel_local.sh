#!/bin/bash
# Example: Local port forwarding to access a website

echo "Example 1: Local Port Forwarding"
echo "================================="
echo ""
echo "This example demonstrates forwarding local port 8080 to example.com:80"
echo ""
echo "Command: sshx -L 8080:example.com:80"
echo ""
echo "Once running, you can access example.com through:"
echo "  curl http://localhost:8080"
echo ""
echo "Or open in your browser:"
echo "  http://localhost:8080"
echo ""

# Uncomment to run (requires sshx built)
# cargo run --release -- -L 8080:example.com:80
