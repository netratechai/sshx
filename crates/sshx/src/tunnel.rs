//! SSH tunnel implementation for local, remote, and dynamic port forwarding.

use anyhow::{anyhow, Context, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// Types of port forwarding supported by SSH tunneling.
#[derive(Debug, Clone)]
pub enum TunnelConfig {
    /// Local port forwarding: -L [bind_address:]port:host:hostport
    /// Listens on local port and forwards to remote host:port through the tunnel.
    Local {
        /// Local address to bind to
        bind_addr: SocketAddr,
        /// Target host to connect to
        target_host: String,
        /// Target port to connect to
        target_port: u16,
    },
    /// Remote port forwarding: -R [bind_address:]port:host:hostport
    /// Server listens on remote port and forwards back to local host:port.
    Remote {
        /// Remote address to bind to
        bind_addr: SocketAddr,
        /// Target host to connect to
        target_host: String,
        /// Target port to connect to
        target_port: u16,
    },
    /// Dynamic port forwarding (SOCKS5 proxy): -D [bind_address:]port
    /// Creates a SOCKS5 proxy server on the specified port.
    Dynamic {
        /// Local address to bind the SOCKS5 proxy to
        bind_addr: SocketAddr,
    },
}

/// Parse a local/remote forwarding specification.
/// Format: [bind_address:]port:host:hostport
pub fn parse_forward_spec(spec: &str) -> Result<(SocketAddr, String, u16)> {
    let parts: Vec<&str> = spec.split(':').collect();
    
    match parts.len() {
        // port:host:hostport
        3 => {
            let port: u16 = parts[0].parse().context("invalid port number")?;
            let host = parts[1].to_string();
            let hostport: u16 = parts[2].parse().context("invalid target port number")?;
            Ok((SocketAddr::from(([127, 0, 0, 1], port)), host, hostport))
        }
        // bind_address:port:host:hostport
        4 => {
            let bind_addr: SocketAddr = format!("{}:{}", parts[0], parts[1])
                .parse()
                .context("invalid bind address")?;
            let host = parts[2].to_string();
            let hostport: u16 = parts[3].parse().context("invalid target port number")?;
            Ok((bind_addr, host, hostport))
        }
        _ => Err(anyhow!(
            "invalid forward specification: expected [bind_address:]port:host:hostport"
        )),
    }
}

/// Parse a dynamic forwarding specification.
/// Format: [bind_address:]port
pub fn parse_dynamic_spec(spec: &str) -> Result<SocketAddr> {
    let parts: Vec<&str> = spec.split(':').collect();
    
    match parts.len() {
        // port
        1 => {
            let port: u16 = parts[0].parse().context("invalid port number")?;
            Ok(SocketAddr::from(([127, 0, 0, 1], port)))
        }
        // bind_address:port
        2 => {
            let bind_addr: SocketAddr = spec.parse().context("invalid bind address")?;
            Ok(bind_addr)
        }
        _ => Err(anyhow!(
            "invalid dynamic forward specification: expected [bind_address:]port"
        )),
    }
}

/// Parse tunnel configurations from command-line arguments.
pub fn parse_tunnel_args(
    local: &[String],
    remote: &[String],
    dynamic: &[String],
) -> Result<Vec<TunnelConfig>> {
    let mut configs = Vec::new();

    // Parse local forwarding specs
    for spec in local {
        let (bind_addr, target_host, target_port) = parse_forward_spec(spec)
            .with_context(|| format!("failed to parse local forward spec: {}", spec))?;
        configs.push(TunnelConfig::Local {
            bind_addr,
            target_host,
            target_port,
        });
    }

    // Parse remote forwarding specs
    for spec in remote {
        let (bind_addr, target_host, target_port) = parse_forward_spec(spec)
            .with_context(|| format!("failed to parse remote forward spec: {}", spec))?;
        configs.push(TunnelConfig::Remote {
            bind_addr,
            target_host,
            target_port,
        });
    }

    // Parse dynamic forwarding specs
    for spec in dynamic {
        let bind_addr = parse_dynamic_spec(spec)
            .with_context(|| format!("failed to parse dynamic forward spec: {}", spec))?;
        configs.push(TunnelConfig::Dynamic { bind_addr });
    }

    Ok(configs)
}

/// Represents a tunnel connection message.
/// 
/// Note: This enum is currently unused but reserved for future protocol integration.
/// The current implementation creates direct TCP connections rather than routing
/// through the sshx protocol. Future versions will use these messages to integrate
/// tunneling with the collaborative terminal session.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TunnelMessage {
    /// Data to be forwarded through the tunnel
    Data {
        /// Unique identifier for the tunnel
        tunnel_id: u32,
        /// Data bytes to forward
        data: Vec<u8>,
    },
    /// Request to open a new tunnel connection
    Open {
        /// Unique identifier for the tunnel
        tunnel_id: u32,
        /// Target host to connect to
        target_host: String,
        /// Target port to connect to
        target_port: u16,
    },
    /// Close an existing tunnel connection
    Close {
        /// Unique identifier for the tunnel
        tunnel_id: u32,
    },
}

/// Local port forwarding task.
/// Listens on a local port and forwards connections through the tunnel.
pub async fn run_local_forward(
    bind_addr: SocketAddr,
    target_host: String,
    target_port: u16,
) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!("Local port forwarding: {} -> {}:{}", bind_addr, target_host, target_port);

    let mut tunnel_id = 0u32;
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                debug!("Accepted connection from {} on local port {}", peer_addr, bind_addr);
                let target_host = target_host.clone();
                let id = tunnel_id;
                tunnel_id = tunnel_id.wrapping_add(1);

                tokio::spawn(async move {
                    if let Err(e) = handle_local_connection(stream, id, target_host, target_port).await {
                        error!("Local forward error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection on {}: {}", bind_addr, e);
            }
        }
    }
}

/// Handle a single local forwarding connection.
async fn handle_local_connection(
    mut stream: TcpStream,
    tunnel_id: u32,
    target_host: String,
    target_port: u16,
) -> Result<()> {
    debug!("Tunnel {} opened to {}:{}", tunnel_id, target_host, target_port);

    // Create a direct connection to the target
    // Note: This creates a direct TCP connection rather than tunneling through
    // the sshx protocol. For full protocol integration, this would need to
    // communicate with the server through the existing gRPC channel.
    let mut target = TcpStream::connect(format!("{}:{}", target_host, target_port)).await?;

    let (mut read_stream, mut write_stream) = stream.split();
    let (mut read_target, mut write_target) = target.split();

    let forward_to_target = async {
        let mut buf = vec![0u8; 8192];
        loop {
            match read_stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    write_target.write_all(&buf[..n]).await?;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok::<_, anyhow::Error>(())
    };

    let forward_from_target = async {
        let mut buf = vec![0u8; 8192];
        loop {
            match read_target.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    write_stream.write_all(&buf[..n]).await?;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok::<_, anyhow::Error>(())
    };

    tokio::select! {
        result = forward_to_target => result?,
        result = forward_from_target => result?,
    }

    debug!("Tunnel {} closed", tunnel_id);

    Ok(())
}

/// Dynamic port forwarding (SOCKS5 proxy) task.
/// Creates a SOCKS5 proxy server on the specified port.
pub async fn run_dynamic_forward(bind_addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!("Dynamic port forwarding (SOCKS5): {}", bind_addr);

    let mut tunnel_id = 0u32;
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                debug!("Accepted SOCKS5 connection from {}", peer_addr);
                let id = tunnel_id;
                tunnel_id = tunnel_id.wrapping_add(1);

                tokio::spawn(async move {
                    if let Err(e) = handle_socks5_connection(stream, id).await {
                        error!("SOCKS5 error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept SOCKS5 connection on {}: {}", bind_addr, e);
            }
        }
    }
}

/// Handle a single SOCKS5 connection.
async fn handle_socks5_connection(mut stream: TcpStream, tunnel_id: u32) -> Result<()> {
    // Read SOCKS5 greeting
    let mut buf = vec![0u8; 2];
    stream.read_exact(&mut buf).await?;

    if buf[0] != 0x05 {
        return Err(anyhow!("Unsupported SOCKS version: {}", buf[0]));
    }

    let nmethods = buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await?;

    // Respond with no authentication required
    stream.write_all(&[0x05, 0x00]).await?;

    // Read connection request
    let mut buf = vec![0u8; 4];
    stream.read_exact(&mut buf).await?;

    if buf[0] != 0x05 {
        return Err(anyhow!("Unsupported SOCKS version in request"));
    }

    let cmd = buf[1];
    if cmd != 0x01 {
        // Only support CONNECT command
        stream.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
        return Err(anyhow!("Unsupported SOCKS command: {}", cmd));
    }

    let atyp = buf[3];
    let (target_host, target_port) = match atyp {
        0x01 => {
            // IPv4
            let mut addr = vec![0u8; 4];
            stream.read_exact(&mut addr).await?;
            let mut port_buf = vec![0u8; 2];
            stream.read_exact(&mut port_buf).await?;
            let port = u16::from_be_bytes([port_buf[0], port_buf[1]]);
            let host = format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]);
            (host, port)
        }
        0x03 => {
            // Domain name
            let mut len_buf = vec![0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let len = len_buf[0] as usize;
            
            // Validate domain name length (max 255 characters per DNS spec)
            if len == 0 || len > 255 {
                stream.write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
                return Err(anyhow!("Invalid domain name length: {}", len));
            }
            
            let mut domain = vec![0u8; len];
            stream.read_exact(&mut domain).await?;
            let mut port_buf = vec![0u8; 2];
            stream.read_exact(&mut port_buf).await?;
            let port = u16::from_be_bytes([port_buf[0], port_buf[1]]);
            let host = String::from_utf8(domain)?;
            (host, port)
        }
        0x04 => {
            // IPv6
            let mut addr = vec![0u8; 16];
            stream.read_exact(&mut addr).await?;
            let mut port_buf = vec![0u8; 2];
            stream.read_exact(&mut port_buf).await?;
            let port = u16::from_be_bytes([port_buf[0], port_buf[1]]);
            let host = format!(
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                u16::from_be_bytes([addr[0], addr[1]]),
                u16::from_be_bytes([addr[2], addr[3]]),
                u16::from_be_bytes([addr[4], addr[5]]),
                u16::from_be_bytes([addr[6], addr[7]]),
                u16::from_be_bytes([addr[8], addr[9]]),
                u16::from_be_bytes([addr[10], addr[11]]),
                u16::from_be_bytes([addr[12], addr[13]]),
                u16::from_be_bytes([addr[14], addr[15]])
            );
            (host, port)
        }
        _ => {
            stream.write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            return Err(anyhow!("Unsupported address type: {}", atyp));
        }
    };

    debug!("SOCKS5 request to {}:{}", target_host, target_port);

    // Try to connect to the target
    match TcpStream::connect(format!("{}:{}", target_host, target_port)).await {
        Ok(mut target) => {
            // Send success response
            stream.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;

            // Forward data bidirectionally
            let (mut read_stream, mut write_stream) = stream.split();
            let (mut read_target, mut write_target) = target.split();

            let forward_to_target = async {
                let mut buf = vec![0u8; 8192];
                loop {
                    match read_stream.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            write_target.write_all(&buf[..n]).await?;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                Ok::<_, anyhow::Error>(())
            };

            let forward_from_target = async {
                let mut buf = vec![0u8; 8192];
                loop {
                    match read_target.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            write_stream.write_all(&buf[..n]).await?;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                Ok::<_, anyhow::Error>(())
            };

            tokio::select! {
                result = forward_to_target => result?,
                result = forward_from_target => result?,
            }

            debug!("SOCKS5 tunnel {} closed", tunnel_id);
        }
        Err(e) => {
            warn!("Failed to connect to {}:{}: {}", target_host, target_port, e);
            // Send connection refused error
            stream.write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            return Err(e.into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_forward_spec() {
        // Test 3-part format: port:host:hostport
        let (addr, host, port) = parse_forward_spec("8080:example.com:80").unwrap();
        assert_eq!(addr.port(), 8080);
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);

        // Test 4-part format: bind_address:port:host:hostport
        let (addr, host, port) = parse_forward_spec("0.0.0.0:8080:example.com:443").unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:8080");
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_parse_dynamic_spec() {
        // Test single port format
        let addr = parse_dynamic_spec("1080").unwrap();
        assert_eq!(addr.port(), 1080);

        // Test bind_address:port format
        let addr = parse_dynamic_spec("0.0.0.0:1080").unwrap();
        assert_eq!(addr.to_string(), "0.0.0.0:1080");
    }

    #[test]
    fn test_parse_tunnel_args() {
        let local = vec!["8080:example.com:80".to_string()];
        let remote = vec!["9090:localhost:3000".to_string()];
        let dynamic = vec!["1080".to_string()];

        let configs = parse_tunnel_args(&local, &remote, &dynamic).unwrap();
        assert_eq!(configs.len(), 3);

        match &configs[0] {
            TunnelConfig::Local { bind_addr, target_host, target_port } => {
                assert_eq!(bind_addr.port(), 8080);
                assert_eq!(target_host, "example.com");
                assert_eq!(*target_port, 80);
            }
            _ => panic!("Expected Local tunnel config"),
        }

        match &configs[1] {
            TunnelConfig::Remote { bind_addr, target_host, target_port } => {
                assert_eq!(bind_addr.port(), 9090);
                assert_eq!(target_host, "localhost");
                assert_eq!(*target_port, 3000);
            }
            _ => panic!("Expected Remote tunnel config"),
        }

        match &configs[2] {
            TunnelConfig::Dynamic { bind_addr } => {
                assert_eq!(bind_addr.port(), 1080);
            }
            _ => panic!("Expected Dynamic tunnel config"),
        }
    }
}
