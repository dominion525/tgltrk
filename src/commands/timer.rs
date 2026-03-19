use colored::Colorize;

use crate::api::client::{ApiClient, CreateTimeEntryParams, TogglClient};
use crate::cli::TimerAction;
use crate::commands::resolve_workspace_id;
use crate::credentials;
use crate::error::{AppError, Result};
use crate::output;

pub async fn execute(action: TimerAction, json: bool, workspace: Option<i64>) -> Result<()> {
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = TogglClient::new(&cred.api_token)?;
    run(action, json, workspace, &client).await
}

async fn run(
    action: TimerAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    match action {
        TimerAction::Current => current(json, client).await,
        TimerAction::Start {
            description,
            project,
            task,
            tags,
            billable,
        } => {
            let wid = resolve_workspace_id(client, workspace).await?;
            start(
                json,
                wid,
                description,
                project,
                task,
                tags,
                billable,
                client,
            )
            .await
        }
        TimerAction::Stop => stop(json, workspace, client).await,
    }
}

async fn current(json: bool, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    match client.get_current_timer().await? {
        Some(entry) => output::print_result(&entry, json),
        None => {
            println!("{}", "No running timer".yellow());
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn start(
    json: bool,
    workspace_id: i64,
    description: Option<String>,
    project: Option<i64>,
    task: Option<i64>,
    tags: Option<Vec<String>>,
    billable: bool,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let params = CreateTimeEntryParams {
        description,
        project_id: project,
        task_id: task,
        tags: tags.unwrap_or_default(),
        billable,
    };
    let entry = client.create_time_entry(workspace_id, &params).await?;
    if json {
        output::print_result(&entry, true)?;
    } else {
        println!("{} Timer started", "✓".green().bold());
        println!("{entry}");
    }
    Ok(())
}

async fn stop(
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let current = client
        .get_current_timer()
        .await?
        .ok_or_else(|| AppError::NotFound("No running timer".to_string()))?;

    let wid = workspace.unwrap_or(current.workspace_id);
    let entry = client.stop_time_entry(wid, current.id).await?;

    if json {
        output::print_result(&entry, true)?;
    } else {
        println!("{} Timer stopped", "✓".green().bold());
        println!("{entry}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::TimeEntry;
    use chrono::Utc;

    fn make_entry(id: i64, running: bool) -> TimeEntry {
        let now = Utc::now();
        TimeEntry {
            id,
            workspace_id: 1,
            description: Some("Test".to_string()),
            start: now,
            stop: if running { None } else { Some(now) },
            duration: if running { -now.timestamp() } else { 3600 },
            project_id: None,
            task_id: None,
            tags: vec![],
            billable: false,
        }
    }

    #[tokio::test]
    async fn stop_with_no_running_timer_returns_error() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer().returning(|| Ok(None));
        let result = run(TimerAction::Stop, false, Some(1), &mock).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn current_with_no_timer_succeeds() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer().returning(|| Ok(None));
        let result = run(TimerAction::Current, false, None, &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stop_calls_stop_time_entry() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer()
            .returning(|| Ok(Some(make_entry(10, true))));
        mock.expect_stop_time_entry()
            .withf(|wid, eid| *wid == 1 && *eid == 10)
            .returning(|_, _| Ok(make_entry(10, false)));

        let result = run(TimerAction::Stop, false, None, &mock).await;
        assert!(result.is_ok());
    }
}
