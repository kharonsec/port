use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::net::TcpListener;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "port", about = "Port and process manager", version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port number to inspect (if no subcommand is given)
    port: Option<u16>,
}

#[derive(Subcommand)]
enum Commands {
    /// Kill process using a port
    Kill { port: u16 },
    /// List all listening ports
    List,
    /// Find first free port in range
    Free { range: String },
    /// Tail the process output (placeholder/mock for existing processes)
    Watch { port: u16 },
}

fn get_port_owner(port: u16) -> Result<Option<(i32, String)>> {
    let mut inodes = HashMap::new();
    
    // Check TCP
    let tcp = procfs::net::tcp()?;
    for entry in tcp {
        if entry.local_address.port() == port && entry.state == procfs::net::TcpState::Listen {
            inodes.insert(entry.inode, ());
        }
    }
    
    // Check TCP6
    if let Ok(tcp6) = procfs::net::tcp6() {
        for entry in tcp6 {
            if entry.local_address.port() == port && entry.state == procfs::net::TcpState::Listen {
                inodes.insert(entry.inode, ());
            }
        }
    }

    if inodes.is_empty() {
        return Ok(None);
    }

    for process in procfs::process::all_processes()? {
        let process = match process {
            Ok(p) => p,
            Err(_) => continue,
        };

        if let Ok(fds) = process.fd() {
            for fd in fds {
                if let procfs::process::FDTarget::Socket(inode) = fd?.target {
                    if inodes.contains_key(&inode) {
                        let name = process.stat()?.comm;
                        return Ok(Some((process.pid, name)));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn list_ports() -> Result<()> {
    let tcp = procfs::net::tcp()?;
    let tcp6 = procfs::net::tcp6().unwrap_or_default();
    
    let mut listening = HashMap::new();
    for entry in tcp.into_iter().chain(tcp6.into_iter()) {
        if entry.state == procfs::net::TcpState::Listen {
            listening.insert(entry.inode, entry.local_address.port());
        }
    }

    println!("{:<10} {:<10} {:<15} {}", "PORT", "PID", "NAME", "INODE");
    
    for process in procfs::process::all_processes()? {
        let process = match process {
            Ok(p) => p,
            Err(_) => continue,
        };

        if let Ok(fds) = process.fd() {
            for fd in fds {
                if let procfs::process::FDTarget::Socket(inode) = fd?.target {
                    if let Some(port) = listening.get(&inode) {
                        let name = process.stat()?.comm;
                        println!("{:<10} {:<10} {:<15} {}", port, process.pid, name, inode);
                    }
                }
            }
        }
    }
    Ok(())
}

fn kill_port(port: u16) -> Result<()> {
    match get_port_owner(port)? {
        Some((pid, name)) => {
            println!("Killing process '{}' (PID: {}) on port {}...", name, pid, port);
            signal::kill(Pid::from_raw(pid), Signal::SIGTERM)
                .with_context(|| format!("Failed to kill PID {}", pid))?;
            Ok(())
        }
        None => {
            println!("No process found on port {}.", port);
            Ok(())
        }
    }
}

fn find_free_port(range: &str) -> Result<()> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range format. Use 'start-end' (e.g., 3000-3010)");
    }
    let start = u16::from_str(parts[0])?;
    let end = u16::from_str(parts[1])?;

    for port in start..=end {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            println!("{}", port);
            return Ok(());
        }
    }
    anyhow::bail!("No free ports in range {}", range);
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match (cli.command, cli.port) {
        (Some(Commands::Kill { port }), _) => kill_port(port)?,
        (Some(Commands::List), _) => list_ports()?,
        (Some(Commands::Free { range }), _) => find_free_port(&range)?,
        (Some(Commands::Watch { port }), _) => {
            match get_port_owner(port)? {
                Some((pid, _)) => {
                    println!("Watching output of PID {} on port {} (Note: limited for already running processes)...", pid, port);
                    // On Linux, you can try to tail stdout/stderr if they are redirected to files or pipes
                    // But for arbitrary processes, it's hard without strace.
                    // We'll use a simple loop as a placeholder or just explain.
                    println!("Tailing output via /proc/{}/fd/1 (if available)...", pid);
                    std::process::Command::new("tail")
                        .arg("-f")
                        .arg(format!("/proc/{}/fd/1", pid))
                        .status()?;
                }
                None => println!("No process found on port {}.", port),
            }
        }
        (None, Some(port)) => {
            match get_port_owner(port)? {
                Some((pid, name)) => println!("Port {}: {} (PID: {})", port, name, pid),
                None => println!("Port {} is free.", port),
            }
        }
        _ => {
            println!("Usage: port <PORT> or port <COMMAND>");
        }
    }

    Ok(())
}
