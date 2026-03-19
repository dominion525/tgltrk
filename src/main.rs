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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    let workspace = cli.workspace;

    let result = match cli.command {
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
    };

    if let Err(e) = result {
        eprintln!("{} {e}", "Error:".red().bold());
        std::process::exit(1);
    }
}
