#!/bin/bash
# Example: Multiple tunnels simultaneously

echo "Example 3: Multiple Tunnels"
echo "============================"
echo ""
echo "This example creates multiple tunnels at once"
echo ""
echo "Command: sshx -L 8080:example.com:80 -L 8443:example.com:443 -D 1080"
echo ""
echo "This will:"
echo "  1. Forward local 8080 to example.com:80"
echo "  2. Forward local 8443 to example.com:443"
echo "  3. Create SOCKS5 proxy on 1080"
echo ""
echo "Usage:"
echo "  curl http://localhost:8080     # Access HTTP"
echo "  curl https://localhost:8443    # Access HTTPS"
echo "  curl --socks5 localhost:1080 https://google.com  # Use proxy"
echo ""

# Uncomment to run (requires sshx built)
# cargo run --release -- -L 8080:example.com:80 -L 8443:example.com:443 -D 1080
