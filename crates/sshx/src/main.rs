use std::process::ExitCode;

use ansi_term::Color::{Cyan, Fixed, Green};
use anyhow::Result;
use clap::Parser;
use sshx::{controller::Controller, runner::Runner, terminal::get_default_shell, tunnel};
use tokio::signal;
use tracing::error;

/// A secure web-based, collaborative terminal.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address of the remote sshx server.
    #[clap(long, default_value = "https://sshx.io", env = "SSHX_SERVER")]
    server: String,

    /// Local shell command to run in the terminal.
    #[clap(long)]
    shell: Option<String>,

    /// Quiet mode, only prints the URL to stdout.
    #[clap(short, long)]
    quiet: bool,

    /// Session name displayed in the title (defaults to user@hostname).
    #[clap(long)]
    name: Option<String>,

    /// Enable read-only access mode - generates separate URLs for viewers and
    /// editors.
    #[clap(long)]
    enable_readers: bool,

    /// Local port forwarding (like ssh -L). Format: [bind_address:]port:host:hostport
    /// Example: 8080:localhost:80 or 0.0.0.0:8080:example.com:80
    #[clap(short = 'L', long = "local")]
    local_forward: Vec<String>,

    /// Remote port forwarding (like ssh -R). Format: [bind_address:]port:host:hostport
    /// Example: 8080:localhost:3000
    #[clap(short = 'R', long = "remote")]
    remote_forward: Vec<String>,

    /// Dynamic port forwarding / SOCKS proxy (like ssh -D). Format: [bind_address:]port
    /// Example: 1080 or 0.0.0.0:1080
    #[clap(short = 'D', long = "dynamic")]
    dynamic_forward: Vec<String>,
}

fn print_greeting(shell: &str, controller: &Controller) {
    let version_str = match option_env!("CARGO_PKG_VERSION") {
        Some(version) => format!("v{version}"),
        None => String::from("[dev]"),
    };
    if let Some(write_url) = controller.write_url() {
        println!(
            r#"
  {sshx} {version}

  {arr}  Read-only link: {link_v}
  {arr}  Writable link:  {link_e}
  {arr}  Shell:          {shell_v}
"#,
            sshx = Green.bold().paint("sshx"),
            version = Green.paint(&version_str),
            arr = Green.paint("➜"),
            link_v = Cyan.underline().paint(controller.url()),
            link_e = Cyan.underline().paint(write_url),
            shell_v = Fixed(8).paint(shell),
        );
    } else {
        println!(
            r#"
  {sshx} {version}

  {arr}  Link:  {link_v}
  {arr}  Shell: {shell_v}
"#,
            sshx = Green.bold().paint("sshx"),
            version = Green.paint(&version_str),
            arr = Green.paint("➜"),
            link_v = Cyan.underline().paint(controller.url()),
            shell_v = Fixed(8).paint(shell),
        );
    }
}

#[tokio::main]
async fn start(args: Args) -> Result<()> {
    // Parse tunnel configurations if provided
    let tunnel_configs = tunnel::parse_tunnel_args(
        &args.local_forward,
        &args.remote_forward,
        &args.dynamic_forward,
    )?;

    // Start tunnel listeners if any tunnels are configured
    if !tunnel_configs.is_empty() {
        for config in tunnel_configs {
            match config {
                tunnel::TunnelConfig::Local {
                    bind_addr,
                    target_host,
                    target_port,
                } => {
                    tokio::spawn(async move {
                        if let Err(e) =
                            tunnel::run_local_forward(bind_addr, target_host, target_port).await
                        {
                            error!("Local forward error: {}", e);
                        }
                    });
                }
                tunnel::TunnelConfig::Dynamic { bind_addr } => {
                    tokio::spawn(async move {
                        if let Err(e) = tunnel::run_dynamic_forward(bind_addr).await {
                            error!("Dynamic forward error: {}", e);
                        }
                    });
                }
                tunnel::TunnelConfig::Remote {
                    bind_addr,
                    target_host,
                    target_port,
                } => {
                    // Remote forwarding would need server-side support
                    error!(
                        "Remote forwarding not yet fully implemented: {} -> {}:{}",
                        bind_addr, target_host, target_port
                    );
                }
            }
        }
    }

    let shell = match args.shell {
        Some(shell) => shell,
        None => get_default_shell().await,
    };

    let name = args.name.unwrap_or_else(|| {
        let mut name = whoami::username();
        if let Ok(host) = whoami::fallible::hostname() {
            // Trim domain information like .lan or .local
            let host = host.split('.').next().unwrap_or(&host);
            name += "@";
            name += host;
        }
        name
    });

    let runner = Runner::Shell(shell.clone());
    let mut controller = Controller::new(&args.server, &name, runner, args.enable_readers).await?;
    if args.quiet {
        if let Some(write_url) = controller.write_url() {
            println!("{}", write_url);
        } else {
            println!("{}", controller.url());
        }
    } else {
        print_greeting(&shell, &controller);
    }

    let exit_signal = signal::ctrl_c();
    tokio::pin!(exit_signal);
    tokio::select! {
        _ = controller.run() => unreachable!(),
        Ok(()) = &mut exit_signal => (),
    };
    controller.close().await?;

    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();

    let default_level = if args.quiet { "error" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or(default_level.into()))
        .with_writer(std::io::stderr)
        .init();

    match start(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err:?}");
            ExitCode::FAILURE
        }
    }
}
