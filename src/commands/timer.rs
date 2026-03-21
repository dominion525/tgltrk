use colored::Colorize;

use crate::api::client::{ApiClient, CreateTimeEntryParams};
use crate::cli::TimerAction;
use crate::commands::CommandContext;
use crate::error::{AppError, Result};
use crate::models::{ProjectId, TaskId, WorkspaceId};
use crate::output;

pub async fn run(action: TimerAction, ctx: &mut CommandContext<'_, impl ApiClient>) -> Result<()> {
    match action {
        TimerAction::Current => current(ctx).await,
        TimerAction::Start {
            description,
            project,
            task,
            tags,
            billable,
        } => {
            let wid = ctx.resolve_workspace_id().await?;
            start(
                wid,
                description,
                project,
                task.map(TaskId),
                tags,
                billable,
                ctx,
            )
            .await
        }
        TimerAction::Stop => stop(ctx).await,
    }
}

async fn current(ctx: &CommandContext<'_, impl ApiClient>) -> Result<()> {
    match ctx.client.get_current_timer().await? {
        Some(entry) => output::print_result(&mut std::io::stdout(), &entry, ctx.json, ctx.hits()),
        None => {
            if ctx.json {
                output::print_null(&mut std::io::stdout(), ctx.json, ctx.hits())
            } else {
                println!("{}", "No running timer".yellow());
                Ok(())
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn start(
    workspace_id: WorkspaceId,
    description: Option<String>,
    project: Option<i64>,
    task: Option<TaskId>,
    tags: Option<Vec<String>>,
    billable: bool,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let params = CreateTimeEntryParams {
        description,
        project_id: project.map(ProjectId),
        task_id: task,
        tags: tags.unwrap_or_default(),
        billable,
        start: None,
        stop: None,
        duration: None,
    };
    let entry = ctx.client.create_time_entry(workspace_id, &params).await?;
    output::print_success(
        &mut std::io::stdout(),
        &entry,
        ctx.json,
        "Timer started",
        ctx.hits(),
    )
}

async fn stop(ctx: &CommandContext<'_, impl ApiClient>) -> Result<()> {
    let current = ctx
        .client
        .get_current_timer()
        .await?
        .ok_or_else(|| AppError::NotFound("No running timer".to_string()))?;

    let wid = ctx
        .workspace
        .map(WorkspaceId)
        .unwrap_or(current.workspace_id);
    let entry = ctx.client.stop_time_entry(wid, current.id).await?;
    output::print_success(
        &mut std::io::stdout(),
        &entry,
        ctx.json,
        "Timer stopped",
        ctx.hits(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::commands::build_client;
    use crate::models::{TimeEntry, TimeEntryId};
    use chrono::Utc;

    fn make_entry(id: i64, running: bool) -> TimeEntry {
        let now = Utc::now();
        TimeEntry {
            id: TimeEntryId(id),
            workspace_id: WorkspaceId(1),
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
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(TimerAction::Stop, &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn current_with_no_timer_succeeds() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer().returning(|| Ok(None));
        let mut ctx = CommandContext::new(&mock, false, None);
        let result = run(TimerAction::Current, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stop_calls_stop_time_entry() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer()
            .returning(|| Ok(Some(make_entry(10, true))));
        mock.expect_stop_time_entry()
            .withf(|wid, eid| *wid == WorkspaceId(1) && *eid == TimeEntryId(10))
            .returning(|_, _| Ok(make_entry(10, false)));

        let mut ctx = CommandContext::new(&mock, false, None);
        let result = run(TimerAction::Stop, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_creates_time_entry() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(100, true)));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            TimerAction::Start {
                description: Some("Work".to_string()),
                project: Some(5),
                task: None,
                tags: Some(vec!["dev".to_string()]),
                billable: true,
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_with_minimal_params() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(101, true)));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            TimerAction::Start {
                description: None,
                project: None,
                task: None,
                tags: None,
                billable: false,
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_with_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(102, true)));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(
            TimerAction::Start {
                description: Some("json test".to_string()),
                project: None,
                task: None,
                tags: None,
                billable: false,
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn current_with_running_timer() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer()
            .returning(|| Ok(Some(make_entry(50, true))));
        let mut ctx = CommandContext::new(&mock, false, None);
        let result = run(TimerAction::Current, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stop_with_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer()
            .returning(|| Ok(Some(make_entry(10, true))));
        mock.expect_stop_time_entry()
            .returning(|_, _| Ok(make_entry(10, false)));
        let mut ctx = CommandContext::new(&mock, true, None);
        let result = run(TimerAction::Stop, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_start_with_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let _guard = crate::ENV_MUTEX.lock().await;
        let server = MockServer::start().await;
        // SAFETY: env var access serialized by ENV_MUTEX
        unsafe { std::env::set_var("TOGGL_API_TOKEN", "test_token") };

        // Mock for create_time_entry (workspace_id is passed explicitly, no /me needed)
        Mock::given(method("POST"))
            .and(path("/workspaces/1/time_entries"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 100, "workspace_id": 1, "description": "Test",
                "start": "2024-01-01T00:00:00Z", "stop": null,
                "duration": -1, "project_id": null, "task_id": null,
                "tags": [], "billable": false
            })))
            .mount(&server)
            .await;

        let client = build_client(Some(&server.uri())).unwrap();
        let mut ctx = CommandContext::new(&client, false, Some(1));
        let result = run(
            TimerAction::Start {
                description: Some("Test".to_string()),
                project: None,
                task: None,
                tags: None,
                billable: false,
            },
            &mut ctx,
        )
        .await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok(), "execute_start failed: {:?}", result.err());
    }
}
