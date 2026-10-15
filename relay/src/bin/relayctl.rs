//! Operator tool for the relay host. Talks to the running relay over its admin socket.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use closing_relay::admin::Request;
use serde_json::Value;

#[derive(Parser)]
#[command(name = "relayctl", version, about = "Operator tool for the Closing Tracker relay")]
struct Cli {
    /// The relay's admin socket.
    #[arg(long, env = "RELAYCTL_SOCKET", default_value = "/run/closing-relay/admin.sock")]
    socket: PathBuf,
    /// Print the relay's raw JSON answer.
    #[arg(long)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Version, uptime and whether pushes are configured.
    Status,
    /// Phones that registered with the Phase 0 push check, newest first.
    PushTokens,
    /// Send the test alert to a registered phone (the newest unless --to is given).
    PushTest {
        /// First characters of the token, as `push-tokens` shows them.
        #[arg(long)]
        to: Option<String>,
        /// Alert text instead of the standard test alert.
        #[arg(long)]
        text: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let req = match cli.cmd {
        Cmd::Status => Request::Status,
        Cmd::PushTokens => Request::PushTokens,
        Cmd::PushTest { to, text } => Request::PushTest { to, text },
    };
    let answer = match ask(&cli.socket, &req) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("relayctl: {e}");
            return ExitCode::FAILURE;
        }
    };
    if cli.json {
        println!("{answer}");
    } else {
        print_answer(&req, &answer);
    }
    if answer["ok"].as_bool() == Some(true) { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn ask(socket: &PathBuf, req: &Request) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket).map_err(|e| {
        let hint = match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                " (run it as a member of closing-relay-ops: sg closing-relay-ops -c 'relayctl …')"
            }
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => {
                " (is the relay running? systemctl status closing-relay)"
            }
            _ => "",
        };
        format!("can't reach the relay at {}: {e}{hint}", socket.display())
    })?;
    let mut line = serde_json::to_string(req).map_err(|e| e.to_string())?;
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).map_err(|e| e.to_string())?;
    serde_json::from_str(&answer).map_err(|e| format!("unreadable answer from the relay: {e}"))
}

fn print_answer(req: &Request, a: &Value) {
    if a["ok"].as_bool() != Some(true) && a.get("error").is_some() {
        println!("Error: {}", a["error"].as_str().unwrap_or("unknown"));
        return;
    }
    match req {
        Request::Status => println!(
            "Relay {} · up {} · listening on {} · pushes {} · {} push-check phone(s)",
            a["version"].as_str().unwrap_or("?"),
            duration(a["uptime_s"].as_u64().unwrap_or(0)),
            a["listen"].as_str().unwrap_or("?"),
            if a["apns"].as_bool() == Some(true) { "on" } else { "OFF (no [apns] in relay.toml)" },
            a["push_check_tokens"].as_u64().unwrap_or(0),
        ),
        Request::PushTokens => {
            let tokens = a["tokens"].as_array().cloned().unwrap_or_default();
            if tokens.is_empty() {
                println!("No phone has registered yet.");
                return;
            }
            println!(
                "{:<10} {:<11} {:<18} {:<18} LABEL",
                "TOKEN", "ENV", "REGISTERED (UTC)", "LAST SEEN (UTC)"
            );
            for t in tokens {
                println!(
                    "{:<10} {:<11} {:<18} {:<18} {}",
                    t["token_prefix"].as_str().unwrap_or(""),
                    t["env"].as_str().unwrap_or(""),
                    stamp(t["registered_at"].as_i64().unwrap_or(0)),
                    stamp(t["last_seen"].as_i64().unwrap_or(0)),
                    t["label"].as_str().unwrap_or(""),
                );
            }
        }
        Request::PushTest { .. } => {
            let who = format!(
                "{} ({})",
                a["token_prefix"].as_str().unwrap_or("?"),
                a["env"].as_str().unwrap_or("?")
            );
            if a["ok"].as_bool() == Some(true) {
                println!(
                    "Apple accepted the alert for {who}. apns-id {}",
                    a["apns_id"].as_str().unwrap_or("-")
                );
            } else {
                println!(
                    "Apple refused the alert for {who}: {} (HTTP {})",
                    a["reason"].as_str().unwrap_or("no reason given"),
                    a["status"]
                );
            }
        }
    }
}

fn duration(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m {}s", secs % 60)
    }
}

fn stamp(unix: i64) -> String {
    let fmt = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]");
    time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|t| t.format(&fmt).ok())
        .unwrap_or_else(|| "?".into())
}
