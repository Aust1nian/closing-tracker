use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use closing_relay::config::Config;
use closing_relay::{apns, server};
use tokio::signal::unix::{SignalKind, signal};

const DEFAULT_CONFIG: &str = "/etc/closing-relay/relay.toml";

#[derive(Parser)]
#[command(name = "closing-relay", version, about = "Closing Tracker relay")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the relay.
    Serve {
        #[arg(long, default_value = DEFAULT_CONFIG)]
        config: PathBuf,
    },
    /// Check relay.toml and the APNs key, then exit.
    CheckConfig {
        #[arg(long, default_value = DEFAULT_CONFIG)]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let filter = tracing_subscriber::EnvFilter::try_from_env("RELAY_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let log = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::stderr().is_terminal())
        .with_writer(std::io::stderr);
    // journald stamps every line itself.
    if std::env::var_os("JOURNAL_STREAM").is_some() {
        log.without_time().init();
    } else {
        log.init();
    }

    let result = match Cli::parse().cmd {
        Cmd::Serve { config } => serve(config).await,
        Cmd::CheckConfig { config } => check_config(config),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("closing-relay: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn serve(path: PathBuf) -> Result<(), String> {
    let cfg = Config::load(&path)?;
    let running = server::start(cfg).await?;
    let mut term = signal(SignalKind::terminate()).map_err(|e| e.to_string())?;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
    tracing::info!("stopping");
    running.stop().await;
    Ok(())
}

fn check_config(path: PathBuf) -> Result<(), String> {
    let cfg = Config::load(&path)?;
    println!("{}: ok", path.display());
    match &cfg.apns {
        Some(a) => {
            apns::Apns::new(a)?;
            println!("APNs key {} (key ID {}, team {}): signs ok", a.key_path.display(), a.key_id, a.team_id);
        }
        None => println!("no [apns] section: pushes will be off"),
    }
    Ok(())
}
