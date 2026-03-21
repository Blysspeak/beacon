mod config;
mod daemon;
mod git;
mod history;
mod hooks;
mod mailbox;
mod output;
mod poller;
mod providers;
mod queue;
mod telegram;
mod tui;
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
    /// Run persistent daemon (for systemd — watches queue, tracks deploys)
    Daemon,

    /// Notify daemon of a new push (writes to queue, exits immediately)
    Notify {
        /// Repository in owner/repo format (auto-detected from git if omitted)
        #[arg(long)]
        repo: Option<String>,

        /// Branch name (auto-detected if omitted)
        #[arg(long)]
        branch: Option<String>,

        /// Full commit SHA (auto-detected if omitted)
        #[arg(long)]
        commit: Option<String>,
    },

    /// Watch deploy status in foreground (manual use, not for hooks)
    Watch,

    /// Show last deploy status from mailbox
    Status {
        /// Output as JSON for machine parsing
        #[arg(long)]
        json: bool,
    },

    /// Git push + notify daemon to track deploy
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

    /// Interactive deploy dashboard (TUI)
    Tui,

    /// Show deploy history
    Log {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Number of entries to show
        #[arg(short, long, default_value_t = 20)]
        n: usize,

        /// Filter by repo (substring match)
        #[arg(long)]
        repo: Option<String>,
    },

    /// Manage GitHub polling (auto-discover repos from history)
    Poll {
        #[command(subcommand)]
        action: PollAction,
    },

    /// Install Claude Code hooks and systemd service
    Install,

    /// Remove Claude Code hooks and systemd service
    Uninstall,
}

#[derive(Subcommand)]
enum PollAction {
    /// Add a repo to watch list
    Add {
        /// Repository in owner/repo format
        repo: String,
    },
    /// Remove a repo from watch list
    Remove {
        /// Repository in owner/repo format
        repo: String,
    },
    /// List watched repos (configured + auto-discovered)
    List,
    /// Set poll interval in seconds
    Interval {
        /// Interval in seconds
        seconds: u64,
    },
}

#[derive(Subcommand)]
enum RemoteAction {
    /// Connect to Beacon bot (get token from /start in Telegram)
    Connect {
        /// API token from /start in @beacon_github_bot
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
        Commands::Daemon => {
            daemon::run().await?;
        }
        Commands::Notify { repo, branch, commit } => {
            let repo = match repo {
                Some(r) => r,
                None => git::detect_repo()?.full_name(),
            };
            let branch = match branch {
                Some(b) => b,
                None => git::current_branch()?,
            };
            let commit = match commit {
                Some(c) => c,
                None => git::head_commit()?,
            };

            queue::enqueue(&repo, &branch, &commit)?;
        }
        Commands::Watch => {
            do_watch().await?;
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

            // Enqueue for daemon instead of spawning a new process
            let repo = git::detect_repo()?;
            let branch = git::current_branch()?;
            let commit = git::head_commit()?;
            queue::enqueue(&repo.full_name(), &branch, &commit)?;
            println!("  Queued for monitoring.");
        }
        Commands::Tui => {
            tui::run()?;
        }
        Commands::Log { json, n, repo } => {
            let filter = history::HistoryFilter { limit: n, repo };
            let entries = history::read(&filter)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else {
                output::print_history(&entries);
            }
        }
        Commands::Remote { action } => handle_remote(action).await?,
        Commands::Poll { action } => handle_poll(action)?,
        Commands::Install => {
            println!("\n  Setting up Beacon...\n");
            hooks::install_claude_hook()?;
            hooks::install_systemd_service()?;
            println!("\n  Done! Beacon daemon is running.\n");
        }
        Commands::Uninstall => {
            println!("\n  Removing Beacon...\n");
            hooks::uninstall_claude_hook()?;
            hooks::uninstall_systemd_service()?;
            println!("\n  Done.\n");
        }
    }

    Ok(())
}

async fn handle_remote(action: RemoteAction) -> Result<()> {
    match action {
        RemoteAction::Connect { token, api_url } => {
            let token = token.trim().to_string();
            if token.is_empty() {
                anyhow::bail!("Token cannot be empty. Get one from /start in @beacon_github_bot");
            }

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

fn handle_poll(action: PollAction) -> Result<()> {
    match action {
        PollAction::Add { repo } => {
            let mut cfg = config::load()?;
            let poll = cfg.poll.get_or_insert_with(config::PollConfig::default);
            if poll.repos.contains(&repo) {
                println!("\n  Already watching {repo}\n");
            } else {
                poll.repos.push(repo.clone());
                config::save(&cfg)?;
                println!("\n  Added {repo} to watch list");
                println!("  Restart daemon to apply: systemctl --user restart beacon\n");
            }
        }
        PollAction::Remove { repo } => {
            let mut cfg = config::load()?;
            if let Some(poll) = &mut cfg.poll {
                poll.repos.retain(|r| r != &repo);
                config::save(&cfg)?;
                println!("\n  Removed {repo} from watch list\n");
            } else {
                println!("\n  {repo} was not in watch list\n");
            }
        }
        PollAction::List => {
            let cfg = config::load()?;
            let poll = cfg.poll.unwrap_or_default();

            println!("\n  Poll interval: {}s", poll.interval_secs);
            println!("  Auto-discover: {}\n", if poll.auto_discover { "on" } else { "off" });

            if !poll.repos.is_empty() {
                println!("  Configured repos:");
                for r in &poll.repos {
                    println!("    {r}");
                }
                println!();
            }

            if poll.auto_discover {
                match history::unique_repos() {
                    Ok(repos) if !repos.is_empty() => {
                        println!("  Auto-discovered from history:");
                        for r in &repos {
                            if !poll.repos.contains(r) {
                                println!("    {r}");
                            }
                        }
                        println!();
                    }
                    _ => {}
                }
            }
        }
        PollAction::Interval { seconds } => {
            if seconds < 10 {
                anyhow::bail!("Interval too short (min 10s to avoid rate limits)");
            }
            let mut cfg = config::load()?;
            let poll = cfg.poll.get_or_insert_with(config::PollConfig::default);
            poll.interval_secs = seconds;
            config::save(&cfg)?;
            println!("\n  Poll interval set to {seconds}s");
            println!("  Restart daemon to apply: systemctl --user restart beacon\n");
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

    if let Some(remote) = &cfg.remote {
        if status.is_terminal() {
            if let Err(e) = telegram::send_deploy_status(remote, &status).await {
                eprintln!("  Warning: remote notification failed: {e:#}");
            }
        }
    }

    Ok(())
}
