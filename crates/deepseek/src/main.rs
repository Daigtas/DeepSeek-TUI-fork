use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use deepseek_app_server::{AppServerOptions, run, run_stdio};

/// DeepSeek TUI — AI-powered terminal workspace
#[derive(Debug, Parser)]
#[command(
    name = "deepseek",
    about = "AI-powered terminal workspace with daemon, session resume, and swarm orchestration",
    version = env!("CARGO_PKG_VERSION"),
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Run as daemon (fork to background)
    #[arg(short, long, global = true)]
    daemon: bool,

    /// Host to bind/connect (default: 127.0.0.1)
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind/connect (default: 8787)
    #[arg(short, long, global = true, default_value_t = 8787)]
    port: u16,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// PID file for daemon mode
    #[arg(long, global = true)]
    pid_file: Option<PathBuf>,

    /// Auto-shutdown when idle
    #[arg(long, global = true)]
    auto_shutdown_idle: bool,

    /// Idle timeout in seconds (default: 300)
    #[arg(long, global = true, default_value_t = 300)]
    idle_timeout_secs: u64,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the daemon server (HTTP API)
    Serve,
    /// Run in stdio JSON-RPC mode (for IDE/editor integration)
    Stdio,
    /// Show daemon status
    Status,
    /// Show version
    Version,
}

fn daemon_running(host: &str, port: u16) -> bool {
    let url = format!("http://{host}:{port}/healthz");
    reqwest::blocking::get(&url)
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn print_dashboard(host: &str, port: u16) {
    let base = format!("http://{host}:{port}");

    // Status
    if let Ok(resp) = reqwest::blocking::get(format!("{base}/daemon/status")) {
        if let Ok(body) = resp.json::<serde_json::Value>() {
            println!("╔══════════════════════════════════════════╗");
            println!("║        DeepSeek TUI Daemon v{}       ║", env!("CARGO_PKG_VERSION"));
            println!("╠══════════════════════════════════════════╣");
            if let Some(detached) = body.get("detached") {
                println!("║  Detached:    {:<28} ║", detached);
            }
            if let Some(tasks) = body.get("active_tasks") {
                println!("║  Active tasks: {:<25} ║", tasks);
            }
            if let Some(clients) = body.get("connected_clients") {
                println!("║  Clients:     {:<28} ║", clients);
            }
            if let Some(started) = body.get("started_at") {
                let ts = started.as_str().unwrap_or("");
                println!("║  Started:     {:<28} ║", &ts[..28.min(ts.len())]);
            }
            println!("╚══════════════════════════════════════════╝");
        }
    }

    // Resume suggestion
    if let Ok(resp) = reqwest::blocking::get(format!("{base}/daemon/resume")) {
        if let Ok(body) = resp.json::<serde_json::Value>() {
            if let Some(resume) = body.get("resume") {
                let action = resume.get("suggested_action").and_then(|v| v.as_str()).unwrap_or("idle");
                let summary = resume.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                let agents = resume.get("active_agents").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                println!();
                println!("  Status:  {summary}");
                if agents > 0 {
                    println!("  Agents:  {agents} active");
                }
                println!("  Suggestion: /{action}");
                println!();
            }
        }
    }

    println!("  Endpoints:");
    println!("    {base}/healthz          — health check");
    println!("    {base}/daemon/status    — daemon status");
    println!("    {base}/daemon/resume    — resume suggestion");
    println!("    {base}/daemon/progress  — progress log");
    println!("    {base}/swarm/agents     — active agents");
    println!("    {base}/hive/summary     — hive mind summary");
    println!();
    println!("  deepseek.boottify.com     — public HTTPS endpoint");
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cmd = cli.command.unwrap_or(Command::Serve);

    match cmd {
        Command::Serve => {
            // Smart default: if daemon already running, show dashboard instead of starting another
            if !cli.daemon && daemon_running(&cli.host, cli.port) {
                print_dashboard(&cli.host, cli.port);
                return Ok(());
            }

            let listen: SocketAddr = format!("{}:{}", cli.host, cli.port)
                .parse()
                .context("invalid listen address")?;

            // Fork before tokio when daemonizing
            #[cfg(unix)]
            if cli.daemon {
                if daemon_running(&cli.host, cli.port) {
                    eprintln!("[deepseek] daemon already running on {}:{}", cli.host, cli.port);
                    std::process::exit(1);
                }
                use std::io::Write;
                use std::os::unix::io::AsRawFd;
                let pid = unsafe { libc::fork() };
                if pid < 0 { anyhow::bail!("fork failed"); }
                if pid > 0 {
                    if let Some(ref pf) = cli.pid_file {
                        let mut f = std::fs::File::create(pf)?;
                        writeln!(f, "{pid}")?;
                    }
                    eprintln!("[deepseek] daemon started (pid {pid}) on {}:{}", cli.host, cli.port);
                    std::process::exit(0);
                }
                unsafe { libc::setsid(); }
                let devnull = std::fs::File::open("/dev/null")?;
                unsafe {
                    libc::dup2(devnull.as_raw_fd(), libc::STDIN_FILENO);
                    libc::dup2(devnull.as_raw_fd(), libc::STDOUT_FILENO);
                    libc::dup2(devnull.as_raw_fd(), libc::STDERR_FILENO);
                }
                if let Some(ref pf) = cli.pid_file {
                    std::fs::write(pf, std::process::id().to_string())?;
                }
            }

            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("failed to build tokio runtime")?;

            if !cli.daemon {
                eprintln!("[deepseek] starting on {}:{}", cli.host, cli.port);
            }

            rt.block_on(run(AppServerOptions {
                listen,
                config_path: cli.config,
                daemon: cli.daemon,
                pid_file: cli.pid_file,
                auto_shutdown_idle: cli.auto_shutdown_idle,
                idle_timeout_secs: cli.idle_timeout_secs,
            }))
        }
        Command::Stdio => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("failed to build tokio runtime")?;

            rt.block_on(run_stdio(cli.config))
        }
        Command::Status => {
            if !daemon_running(&cli.host, cli.port) {
                eprintln!("No daemon running on {}:{}", cli.host, cli.port);
                eprintln!("Start one with: deepseek serve --daemon");
                std::process::exit(1);
            }
            print_dashboard(&cli.host, cli.port);
            Ok(())
        }
        Command::Version => {
            println!("deepseek v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
