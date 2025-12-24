#!/bin/bash
# Example: SOCKS5 proxy for secure browsing

echo "Example 2: Dynamic Port Forwarding (SOCKS5 Proxy)"
echo "=================================================="
echo ""
echo "This example creates a SOCKS5 proxy on port 1080"
echo ""
echo "Command: sshx -D 1080"
echo ""
echo "Once running, you can use it with:"
echo "  curl --socks5 localhost:1080 https://ipinfo.io"
echo ""
echo "Or configure your browser:"
echo "  Firefox: Settings → Network Settings → Manual proxy"
echo "    SOCKS Host: 127.0.0.1"
echo "    Port: 1080"
echo "    SOCKS v5"
echo ""

# Uncomment to run (requires sshx built)
# cargo run --release -- -D 1080
