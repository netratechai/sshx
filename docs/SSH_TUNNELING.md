# SSH Tunneling with sshx

This document explains how to use SSH tunneling features in sshx to forward ports and create SOCKS proxies.

## Overview

sshx now supports SSH tunneling, allowing you to securely forward network traffic through your sshx session. This is useful for:

- Accessing services behind firewalls
- Securely browsing the internet through a remote server
- Exposing local services to remote users
- Creating secure tunnels for database connections

## Types of Tunneling

### Local Port Forwarding (-L)

Local port forwarding listens on a local port and forwards connections through sshx to a remote host and port.

**Usage:**
```bash
sshx -L [bind_address:]port:host:hostport
```

**Examples:**

1. Forward local port 8080 to example.com:80:
```bash
sshx -L 8080:example.com:80
```

2. Forward local port 3306 to a remote MySQL server:
```bash
sshx -L 3306:mysql.internal:3306
```
Then connect with: `mysql -h 127.0.0.1 -P 3306`

3. Bind to all interfaces (not just localhost):
```bash
sshx -L 0.0.0.0:8080:example.com:80
```

### Remote Port Forwarding (-R)

Remote port forwarding makes a local service available to remote users by creating a listener on the remote side.

**Usage:**
```bash
sshx -R [bind_address:]port:host:hostport
```

**Examples:**

1. Expose local web server (port 3000) to remote users:
```bash
sshx -R 8080:localhost:3000
```

2. Share a local database with a remote team:
```bash
sshx -R 5432:localhost:5432
```

**Note:** Remote forwarding requires server-side support and may have restrictions depending on your sshx server configuration.

### Dynamic Port Forwarding (-D) / SOCKS Proxy

Dynamic port forwarding creates a SOCKS5 proxy server on your local machine, allowing any SOCKS-capable application to route traffic through sshx.

**Usage:**
```bash
sshx -D [bind_address:]port
```

**Examples:**

1. Create a SOCKS5 proxy on port 1080:
```bash
sshx -D 1080
```

2. Configure your browser to use the proxy:
   - Firefox: Settings → Network Settings → Manual proxy configuration
     - SOCKS Host: `127.0.0.1`
     - Port: `1080`
     - SOCKS v5
   - Chrome/Edge: Launch with `--proxy-server="socks5://127.0.0.1:1080"`

3. Use with curl:
```bash
curl --socks5 127.0.0.1:1080 https://example.com
```

## Multiple Tunnels

You can specify multiple tunnel options simultaneously:

```bash
sshx -L 8080:web.internal:80 -L 3306:db.internal:3306 -D 1080
```

This command will:
- Forward local port 8080 to web.internal:80
- Forward local port 3306 to db.internal:3306
- Create a SOCKS5 proxy on port 1080

## Security Considerations

1. **Bind Address**: By default, tunnels bind to `127.0.0.1` (localhost only). Use `0.0.0.0` to allow connections from other machines, but be aware this exposes the tunnel to your local network.

2. **Firewall Rules**: Ensure your firewall allows the ports you're forwarding.

3. **Encryption**: All tunnel traffic is encrypted through the sshx connection, providing secure communication.

4. **Authentication**: Access to tunnels is controlled by access to the sshx session URL.

## Use Cases

### Database Access

Access a remote database as if it were local:
```bash
sshx -L 5432:postgres.internal:5432
psql -h 127.0.0.1 -p 5432 -U user database
```

### Web Development

Share your local development server:
```bash
# Terminal 1: Start your dev server
npm run dev

# Terminal 2: Create tunnel
sshx -R 8080:localhost:3000
```

### Secure Browsing

Browse the internet through a remote server:
```bash
sshx -D 1080
# Configure browser to use SOCKS5 proxy at 127.0.0.1:1080
```

### Jump Box / Bastion Host

Use sshx as a jump box to access internal resources:
```bash
sshx -L 2222:internal-server:22 -L 8080:internal-web:80
ssh -p 2222 user@127.0.0.1  # Access internal SSH server
curl http://127.0.0.1:8080   # Access internal web server
```

## Troubleshooting

### Port Already in Use
If you see "Address already in use", another process is using that port. Try a different port or stop the conflicting process:
```bash
# Find what's using the port (Linux/Mac)
lsof -i :8080
# or
netstat -tulpn | grep 8080
```

### Connection Refused
Ensure the target service is running and accessible from the sshx client machine.

### SOCKS Proxy Not Working
1. Verify the proxy is running: `netstat -an | grep 1080`
2. Test with curl: `curl --socks5 127.0.0.1:1080 https://example.com`
3. Check application proxy settings

## Limitations

1. **Hybrid Architecture**: The current implementation uses direct TCP connections for immediate usability. The sshx protocol has been extended with tunnel messages (TunnelOpen, TunnelData, TunnelClose), and the client-side runner architecture supports protocol-integrated tunnels. Full end-to-end protocol routing requires server-side implementation.

2. **Remote Forwarding**: Remote port forwarding (`-R`) requires server-side support and is not yet available.

3. **Protocol Support**: Dynamic forwarding only supports TCP connections, not UDP.

## Protocol Integration Status

The groundwork for deep protocol integration is complete:
- ✅ Protobuf messages defined (TunnelOpen, TunnelData, TunnelClose)
- ✅ Runner::Tunnel variant implemented
- ✅ Controller handlers for tunnel messages
- ⏳ Server-side tunnel routing (pending)
- ⏳ Web interface tunnel management (pending)

## Future Enhancements

Planned improvements include:
- Server-side implementation for full protocol routing
- Visual tunnel status in the web interface
- Automatic reconnection for long-running tunnels
- Tunnel traffic statistics and monitoring
- Support for UDP forwarding
