mod api;
mod cache;
mod cli;
mod commands;
mod constants;
mod credentials;
mod error;
mod models;
mod output;

use clap::Parser;
use cli::{CacheAction, Cli, Command};
use colored::Colorize;

pub async fn run_cli(cli: Cli) -> error::Result<()> {
    let json = cli.json;
    let workspace = cli.workspace;

    match cli.command {
        Command::Auth { action } => commands::auth::execute(action).await,
        Command::Me => commands::me::execute(json, workspace).await,
        Command::Timer { action } => commands::timer::execute(action, json, workspace).await,
        Command::Entries { action } => commands::entries::execute(action, json, workspace).await,
        Command::Projects { action } => commands::projects::execute(action, json, workspace).await,
        Command::Tags { action } => commands::tags::execute(action, json, workspace).await,
        Command::Cache { action } => match action {
            CacheAction::Clear => commands::cache_cmd::clear().await,
            CacheAction::Status => commands::cache_cmd::status().await,
        },
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run_cli(cli).await {
        eprintln!("{} {e}", "Error:".red().bold());
        std::process::exit(1);
    }
}

/// Mutex to serialize tests that modify environment variables (e.g. TOGGL_API_TOKEN).
/// Without this, parallel test threads race on set_var/remove_var.
#[cfg(test)]
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_cli_cache_status_succeeds() {
        let cli = Cli {
            command: Command::Cache {
                action: CacheAction::Status,
            },
            json: false,
            workspace: None,
        };
        let result = run_cli(cli).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_cli_cache_clear_succeeds() {
        let cli = Cli {
            command: Command::Cache {
                action: CacheAction::Clear,
            },
            json: false,
            workspace: None,
        };
        let result = run_cli(cli).await;
        assert!(result.is_ok());
    }
}
