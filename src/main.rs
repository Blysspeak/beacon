mod config;
mod git;
mod hooks;
mod mailbox;
mod output;
mod providers;
mod telegram;
mod watcher;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use providers::github::{GitHubProvider, resolve_token};

#[derive(Parser)]
#[command(
    name = "beacon",
    version,
    about = "Monitor CI/CD deploy status after git push"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch deploy status after push (foreground by default)
    Watch {
        /// Run in background as daemon
        #[arg(long)]
        daemon: bool,
    },

    /// Show last deploy status from mailbox
    Status {
        /// Output as JSON for machine parsing
        #[arg(long)]
        json: bool,
    },

    /// Git push + auto-watch deploy
    Push {
        /// Arguments forwarded to git push
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Manage remote Telegram notifications
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },

    /// Install Claude Code hooks for automatic deploy monitoring
    Install,

    /// Remove Claude Code hooks
    Uninstall,
}

#[derive(Subcommand)]
enum RemoteAction {
    /// Connect to Beacon bot (get token from /start in Telegram)
    Connect {
        /// API token from /start in @BeaconCIBot
        token: String,

        /// Custom API server URL
        #[arg(long)]
        api_url: Option<String>,
    },

    /// Disconnect remote notifications
    Disconnect,

    /// Send a test notification to verify connection
    Test,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch { daemon } => {
            if daemon {
                watcher::daemonize()?;
            } else {
                do_watch().await?;
            }
        }
        Commands::Status { json } => {
            match mailbox::read_last()? {
                Some(s) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&s)?);
                    } else {
                        output::print_status(&s);
                    }
                }
                None => {
                    if json {
                        println!("null");
                    } else {
                        println!("\n  No deploy status found. Run `beacon watch` after a push.\n");
                    }
                }
            }
        }
        Commands::Push { args } => {
            let mut cmd = std::process::Command::new("git");
            cmd.arg("push");
            for arg in &args {
                cmd.arg(arg);
            }

            let exit = cmd.status().context("failed to run git push")?;
            if !exit.success() {
                anyhow::bail!("git push failed (exit {})", exit);
            }

            do_watch().await?;
        }
        Commands::Remote { action } => handle_remote(action).await?,
        Commands::Install => {
            println!("\n  Setting up Claude Code integration...\n");
            hooks::install_claude_hook()?;
            println!("\n  Done! Beacon will now auto-monitor deploys after git push.\n");
        }
        Commands::Uninstall => {
            println!("\n  Removing Claude Code integration...\n");
            hooks::uninstall_claude_hook()?;
            println!("\n  Done.\n");
        }
    }

    Ok(())
}

async fn handle_remote(action: RemoteAction) -> Result<()> {
    match action {
        RemoteAction::Connect { token, api_url } => {
            let url = api_url.unwrap_or_else(|| config::DEFAULT_API_URL.to_string());

            let mut cfg = config::load()?;
            cfg.remote = Some(config::RemoteConfig {
                token: token.clone(),
                api_url: url,
            });
            config::save(&cfg)?;

            let preview = &token[..token.len().min(8)];
            println!("\n  Connected (token: {preview}...)");
            println!("  Run `beacon remote test` to verify.\n");
        }
        RemoteAction::Disconnect => {
            let mut cfg = config::load()?;
            cfg.remote = None;
            config::save(&cfg)?;
            println!("\n  Remote disconnected.\n");
        }
        RemoteAction::Test => {
            let cfg = config::load()?;
            match cfg.remote {
                Some(remote) => {
                    telegram::send_test(&remote).await?;
                    println!("\n  Test message sent! Check your Telegram.\n");
                }
                None => {
                    println!("\n  Not connected. Run `beacon remote connect <TOKEN>` first.\n");
                }
            }
        }
    }
    Ok(())
}

async fn do_watch() -> Result<()> {
    let repo = git::detect_repo()?;
    let branch = git::current_branch()?;
    let commit = git::head_commit()?;
    let token = resolve_token()?;
    let cfg = config::load()?;

    let provider = GitHubProvider::new(&token)?;
    let status = watcher::watch(provider, &repo, &branch, &commit).await?;

    // Send to remote if connected and deploy is terminal
    if let Some(remote) = &cfg.remote {
        if status.is_terminal() {
            if let Err(e) = telegram::send_deploy_status(remote, &status).await {
                eprintln!("  Warning: remote notification failed: {e:#}");
            }
        }
    }

    Ok(())
}
