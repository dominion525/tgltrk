use colored::Colorize;

use crate::api::client::{ApiClient, CreateTimeEntryParams};
use crate::cli::TimerAction;
use crate::commands::{build_client, resolve_workspace_id, CacheHits};
use crate::error::{AppError, Result};
use crate::output;

pub async fn execute(action: TimerAction, json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(action, json, workspace, None).await
}

pub async fn execute_with_base_url(
    action: TimerAction,
    json: bool,
    workspace: Option<i64>,
    base_url: Option<&str>,
) -> Result<()> {
    let client = build_client(base_url)?;
    run(action, json, workspace, &client).await
}

async fn run(
    action: TimerAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let mut hits = CacheHits::new();
    match action {
        TimerAction::Current => current(json, client, &hits).await,
        TimerAction::Start {
            description,
            project,
            task,
            tags,
            billable,
        } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            start(
                json,
                wid,
                description,
                project,
                task,
                tags,
                billable,
                client,
                &hits,
            )
            .await
        }
        TimerAction::Stop => stop(json, workspace, client, &hits).await,
    }
}

async fn current(
    json: bool,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    match client.get_current_timer().await? {
        Some(entry) => output::print_result(&entry, json, hits),
        None => {
            if json {
                output::print_null(json, hits)
            } else {
                println!("{}", "No running timer".yellow());
                Ok(())
            }
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
    hits: &CacheHits,
) -> Result<()> {
    let params = CreateTimeEntryParams {
        description,
        project_id: project,
        task_id: task,
        tags: tags.unwrap_or_default(),
        billable,
    };
    let entry = client.create_time_entry(workspace_id, &params).await?;
    output::print_success(&entry, json, "Timer started", hits)
}

async fn stop(
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let current = client
        .get_current_timer()
        .await?
        .ok_or_else(|| AppError::NotFound("No running timer".to_string()))?;

    let wid = workspace.unwrap_or(current.workspace_id);
    let entry = client.stop_time_entry(wid, current.id).await?;
    output::print_success(&entry, json, "Timer stopped", hits)
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

    #[tokio::test]
    async fn start_creates_time_entry() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(100, true)));
        let result = run(
            TimerAction::Start {
                description: Some("Work".to_string()),
                project: Some(5),
                task: None,
                tags: Some(vec!["dev".to_string()]),
                billable: true,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_with_minimal_params() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(101, true)));
        let result = run(
            TimerAction::Start {
                description: None,
                project: None,
                task: None,
                tags: None,
                billable: false,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_with_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(102, true)));
        let result = run(
            TimerAction::Start {
                description: Some("json test".to_string()),
                project: None,
                task: None,
                tags: None,
                billable: false,
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn current_with_running_timer() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer()
            .returning(|| Ok(Some(make_entry(50, true))));
        let result = run(TimerAction::Current, false, None, &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stop_with_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_current_timer()
            .returning(|| Ok(Some(make_entry(10, true))));
        mock.expect_stop_time_entry()
            .returning(|_, _| Ok(make_entry(10, false)));
        let result = run(TimerAction::Stop, true, None, &mock).await;
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

        // Mock for get_me (workspace resolution)
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "email": "t@t.com", "fullname": "T",
                "default_workspace_id": 1, "timezone": "UTC"
            })))
            .mount(&server)
            .await;

        // Mock for create_time_entry
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

        let result = execute_with_base_url(
            TimerAction::Start {
                description: Some("Test".to_string()),
                project: None,
                task: None,
                tags: None,
                billable: false,
            },
            false,
            None,
            Some(&server.uri()),
        )
        .await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
