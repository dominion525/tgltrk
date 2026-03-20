use chrono::NaiveDate;
use clap::{Parser, Subcommand};

fn parse_date(s: &str) -> Result<String, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("Invalid date format: '{s}' (expected YYYY-MM-DD)"))?;
    Ok(s.to_string())
}

#[derive(Parser)]
#[command(
    name = "tgltrk",
    about = "Toggl Track CLI",
    version,
    after_help = "AI agents: run `tgltrk --help-skill` for structured usage instructions."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Override workspace ID
    #[arg(long, global = true)]
    pub workspace: Option<i64>,

    /// Print SKILL.md for AI agent integration and exit
    #[arg(long = "help-skill", visible_aliases = ["export-skill", "skill"])]
    pub help_skill: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Show current user info
    Me,
    /// Manage running timer
    Timer {
        #[command(subcommand)]
        action: TimerAction,
    },
    /// Manage time entries
    Entries {
        #[command(subcommand)]
        action: EntriesAction,
    },
    /// Manage projects
    Projects {
        #[command(subcommand)]
        action: ProjectsAction,
    },
    /// Manage tags
    Tags {
        #[command(subcommand)]
        action: TagsAction,
    },
    /// Manage clients
    Clients {
        #[command(subcommand)]
        action: ClientsAction,
    },
    /// Manage cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Save API token
    Login {
        /// Toggl Track API token (if omitted, reads from stdin)
        token: Option<String>,
    },
    /// Remove saved API token
    Clear,
    /// Show authentication status
    Status,
}

#[derive(Subcommand)]
pub enum TimerAction {
    /// Show current running timer
    Current,
    /// Start a new timer
    Start {
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Project ID
        #[arg(short, long)]
        project: Option<i64>,
        /// Task ID
        #[arg(long)]
        task: Option<i64>,
        /// Tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// Billable
        #[arg(short, long)]
        billable: bool,
    },
    /// Stop the current timer
    Stop,
}

#[derive(Subcommand)]
pub enum EntriesAction {
    /// List recent time entries
    List {
        /// Start date (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date)]
        since: Option<String>,
        /// End date (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date)]
        until: Option<String>,
        /// Number of entries to show
        #[arg(short = 'n', long)]
        count: Option<usize>,
    },
    /// Get a specific time entry
    Get {
        /// Time entry ID
        id: i64,
    },
    /// Create a time entry with specific start/stop times
    Create {
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Project ID
        #[arg(short, long)]
        project: Option<i64>,
        /// Task ID
        #[arg(long)]
        task: Option<i64>,
        /// Tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// Billable
        #[arg(short, long)]
        billable: bool,
        /// Start time (e.g. "2026-03-20T09:00", "2026-03-20 09:00")
        #[arg(long)]
        start: String,
        /// Stop time (provide --stop or --duration, not both)
        #[arg(long)]
        stop: Option<String>,
        /// Duration (e.g. "1h30m", "90m", "5400")
        #[arg(long)]
        duration: Option<String>,
    },
    /// Edit a time entry
    Edit {
        /// Time entry ID
        id: i64,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Project ID
        #[arg(short, long)]
        project: Option<i64>,
        /// Tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// Billable
        #[arg(short, long)]
        billable: Option<bool>,
        /// Start time
        #[arg(long)]
        start: Option<String>,
        /// Stop time
        #[arg(long)]
        stop: Option<String>,
        /// Duration (e.g. "1h30m", "90m", "5400")
        #[arg(long)]
        duration: Option<String>,
    },
    /// Delete a time entry
    Delete {
        /// Time entry ID
        id: i64,
    },
    /// Continue a previous time entry
    Continue {
        /// Time entry ID to continue
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum ProjectsAction {
    /// List all projects
    List,
    /// Get a specific project
    Get {
        /// Project ID
        id: i64,
    },
    /// Create a new project
    Create {
        /// Project name
        name: String,
        /// Client ID
        #[arg(long)]
        client: Option<i64>,
    },
    /// Update a project
    Update {
        /// Project ID
        id: i64,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// Client ID
        #[arg(long)]
        client: Option<i64>,
    },
    /// Delete a project
    Delete {
        /// Project ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum TagsAction {
    /// List all tags
    List,
    /// Create a new tag
    Create {
        /// Tag name
        name: String,
    },
    /// Update a tag
    Update {
        /// Tag ID
        id: i64,
        /// New name
        #[arg(long)]
        name: String,
    },
    /// Delete a tag
    Delete {
        /// Tag ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum ClientsAction {
    /// List all clients
    List,
    /// Create a new client
    Create {
        /// Client name
        name: String,
    },
    /// Update a client
    Update {
        /// Client ID
        id: i64,
        /// New name
        #[arg(long)]
        name: String,
    },
    /// Delete a client
    Delete {
        /// Client ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear all cached data
    Clear,
    /// Show cache status
    Status,
}
