use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use deepseek_app_server::{AppServerOptions, run};

#[derive(Debug, Parser)]
#[command(
    name = "deepseek-app-server",
    about = "Run the DeepSeek app-server transport"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 8787)]
    port: u16,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    daemon: bool,
    #[arg(long)]
    pid_file: Option<PathBuf>,
    #[arg(long)]
    auto_shutdown_idle: bool,
    #[arg(long, default_value_t = 300)]
    idle_timeout_secs: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Daemonize BEFORE creating tokio runtime
    #[cfg(unix)]
    if cli.daemon {
        use std::io::Write;
        use std::os::unix::io::AsRawFd;
        let pid = unsafe { libc::fork() };
        if pid < 0 { anyhow::bail!("fork failed"); }
        if pid > 0 {
            if let Some(ref pf) = cli.pid_file {
                let mut f = std::fs::File::create(pf)?;
                writeln!(f, "{pid}")?;
            }
            eprintln!("[deepseek-app-server] daemon started (pid {pid})");
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

    let listen: SocketAddr = format!("{}:{}", cli.host, cli.port)
        .parse()
        .with_context(|| format!("invalid listen address {}:{}", cli.host, cli.port))?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    rt.block_on(run(AppServerOptions {
        listen,
        config_path: cli.config,
        daemon: cli.daemon,
        pid_file: cli.pid_file,
        auto_shutdown_idle: cli.auto_shutdown_idle,
        idle_timeout_secs: cli.idle_timeout_secs,
    }))
}
