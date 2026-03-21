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
    if cli.help_skill {
        print!("{}", include_str!("../SKILL.md"));
        return Ok(());
    }

    let command = cli.command.ok_or_else(|| {
        error::AppError::InvalidInput("No command provided. Run with --help for usage.".to_string())
    })?;

    let json = cli.json;
    let workspace = cli.workspace;

    match command {
        Command::Auth { action } => commands::auth::execute(action, json).await,
        Command::Cache { action } => match action {
            CacheAction::Clear => commands::cache_cmd::clear(json).await,
            CacheAction::Status => commands::cache_cmd::status(json).await,
        },
        cmd => {
            let client = commands::build_client(None)?;
            let mut ctx = commands::CommandContext::new(&client, json, workspace);
            match cmd {
                Command::Me => commands::me::run(&mut ctx).await,
                Command::Timer { action } => commands::timer::run(action, &mut ctx).await,
                Command::Entries { action } => commands::entries::run(action, &mut ctx).await,
                Command::Projects { action } => commands::projects::run(action, &mut ctx).await,
                Command::Tags { action } => commands::tags::run(action, &mut ctx).await,
                Command::Clients { action } => commands::clients::run(action, &mut ctx).await,
                Command::Workspaces { action } => {
                    commands::workspaces::run(action, &mut ctx).await
                }
                Command::Auth { .. } | Command::Cache { .. } => unreachable!(),
            }
        }
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
pub(crate) static ENV_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_cli_cache_status_succeeds() {
        let cli = Cli {
            command: Some(Command::Cache {
                action: CacheAction::Status,
            }),
            json: false,
            workspace: None,
            help_skill: false,
        };
        let result = run_cli(cli).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_cli_cache_clear_succeeds() {
        let cli = Cli {
            command: Some(Command::Cache {
                action: CacheAction::Clear,
            }),
            json: false,
            workspace: None,
            help_skill: false,
        };
        let result = run_cli(cli).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_cli_help_skill_outputs_skill_md() {
        let cli = Cli {
            command: None,
            json: false,
            workspace: None,
            help_skill: true,
        };
        let result = run_cli(cli).await;
        assert!(result.is_ok());
    }
}
