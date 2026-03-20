use chrono::{DateTime, Local, NaiveDateTime, Utc};

use crate::api::client::{ApiClient, CreateTimeEntryParams, UpdateTimeEntryParams};
use crate::cli::EntriesAction;
use crate::commands::{CacheHits, build_client, resolve_workspace_id};
use crate::error::{AppError, Result};
use crate::models::{TaskId, TimeEntryId, WorkspaceId};
use crate::output;

fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    // Try UTC format first: "2026-03-20T09:00:00Z"
    if let Ok(dt) = s.parse::<DateTime<Utc>>() {
        return Ok(dt);
    }
    // Try local time formats: "2026-03-20T09:00" or "2026-03-20 09:00"
    let normalized = s.replace(' ', "T");
    let naive = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| {
            AppError::InvalidInput(format!(
                "Invalid datetime: '{s}' (expected YYYY-MM-DD HH:MM or YYYY-MM-DDTHH:MM)"
            ))
        })?;
    let local = naive
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| AppError::InvalidInput(format!("Ambiguous local time: '{s}'")))?;
    Ok(local.with_timezone(&Utc))
}

fn parse_duration_str(s: &str) -> Result<i64> {
    let s = s.trim();
    // Pure number → seconds
    if let Ok(secs) = s.parse::<i64>() {
        return Ok(secs);
    }
    let mut total: i64 = 0;
    let mut num_buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let n: i64 = num_buf.parse().map_err(|_| {
                AppError::InvalidInput(format!("Invalid duration: '{s}'"))
            })?;
            num_buf.clear();
            let overflow_err =
                || AppError::InvalidInput(format!("Duration overflow: '{s}'"));
            match ch {
                'h' | 'H' => {
                    total = total
                        .checked_add(n.checked_mul(3600).ok_or_else(overflow_err)?)
                        .ok_or_else(overflow_err)?;
                }
                'm' | 'M' => {
                    total = total
                        .checked_add(n.checked_mul(60).ok_or_else(overflow_err)?)
                        .ok_or_else(overflow_err)?;
                }
                's' | 'S' => {
                    total = total.checked_add(n).ok_or_else(overflow_err)?;
                }
                _ => {
                    return Err(AppError::InvalidInput(format!(
                        "Invalid duration unit '{ch}' in '{s}' (expected h, m, or s)"
                    )))
                }
            }
        }
    }
    if !num_buf.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Invalid duration: '{s}' (trailing number without unit, use e.g. '90m' or '1h30m')"
        )));
    }
    if total == 0 {
        return Err(AppError::InvalidInput(format!("Duration must be positive: '{s}'")));
    }
    Ok(total)
}

pub async fn execute(action: EntriesAction, json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(action, json, workspace, None).await
}

pub async fn execute_with_base_url(
    action: EntriesAction,
    json: bool,
    workspace: Option<i64>,
    base_url: Option<&str>,
) -> Result<()> {
    let client = build_client(base_url)?;
    run(action, json, workspace, &client).await
}

async fn run(
    action: EntriesAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let mut hits = CacheHits::new();
    match action {
        EntriesAction::List {
            since,
            until,
            count,
        } => list(json, since, until, count, client, &hits).await,
        EntriesAction::Get { id } => get(json, TimeEntryId(id), client, &hits).await,
        EntriesAction::Create {
            description,
            project,
            task,
            tags,
            billable,
            start,
            stop,
            duration,
        } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            create(
                json, wid, description, project, task, tags, billable, start, stop, duration,
                client, &hits,
            )
            .await
        }
        EntriesAction::Edit {
            id,
            description,
            project,
            tags,
            billable,
            start,
            stop,
            duration,
        } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            edit(
                json, wid, TimeEntryId(id), description, project, tags, billable, start, stop,
                duration, client, &hits,
            )
            .await
        }
        EntriesAction::Delete { id } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            delete(json, wid, TimeEntryId(id), client, &hits).await
        }
        EntriesAction::Continue { id } => {
            continue_entry(json, TimeEntryId(id), client, &hits).await
        }
    }
}

async fn list(
    json: bool,
    since: Option<String>,
    until: Option<String>,
    count: Option<usize>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let mut entries = client.get_time_entries(since, until).await?;
    entries.sort_by_key(|e| std::cmp::Reverse(e.start));
    if let Some(n) = count {
        entries.truncate(n);
    }
    output::print_list(&mut std::io::stdout(), &entries, json, hits)
}

async fn get(
    json: bool,
    id: TimeEntryId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let entry = client.get_time_entry(id).await?;
    output::print_result(&mut std::io::stdout(), &entry, json, hits)
}

#[allow(clippy::too_many_arguments)]
async fn create(
    json: bool,
    workspace_id: WorkspaceId,
    description: Option<String>,
    project: Option<i64>,
    task: Option<i64>,
    tags: Option<Vec<String>>,
    billable: bool,
    start_str: String,
    stop_str: Option<String>,
    duration_str: Option<String>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let start = parse_datetime(&start_str)?;
    let (stop, duration) = match (stop_str, duration_str) {
        (Some(s), None) => {
            let stop = parse_datetime(&s)?;
            let dur = (stop - start).num_seconds();
            if dur <= 0 {
                return Err(AppError::InvalidInput(
                    "stop must be after start".to_string(),
                ));
            }
            (Some(stop), Some(dur))
        }
        (None, Some(d)) => {
            let dur = parse_duration_str(&d)?;
            let stop = start + chrono::TimeDelta::seconds(dur);
            (Some(stop), Some(dur))
        }
        (None, None) => {
            return Err(AppError::InvalidInput(
                "--stop or --duration is required for entries create".to_string(),
            ))
        }
        (Some(_), Some(_)) => {
            return Err(AppError::InvalidInput(
                "specify --stop or --duration, not both".to_string(),
            ))
        }
    };
    let params = CreateTimeEntryParams {
        description,
        project_id: project,
        task_id: task.map(TaskId),
        tags: tags.unwrap_or_default(),
        billable,
        start: Some(start),
        stop,
        duration,
    };
    let entry = client.create_time_entry(workspace_id, &params).await?;
    output::print_success(&mut std::io::stdout(), &entry, json, "Entry created", hits)
}

#[allow(clippy::too_many_arguments)]
async fn edit(
    json: bool,
    workspace_id: WorkspaceId,
    entry_id: TimeEntryId,
    description: Option<String>,
    project: Option<i64>,
    tags: Option<Vec<String>>,
    billable: Option<bool>,
    start_str: Option<String>,
    stop_str: Option<String>,
    duration_str: Option<String>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    if stop_str.is_some() && duration_str.is_some() {
        return Err(AppError::InvalidInput(
            "specify --stop or --duration, not both".to_string(),
        ));
    }
    let start = start_str.map(|s| parse_datetime(&s)).transpose()?;
    let stop = stop_str.map(|s| parse_datetime(&s)).transpose()?;
    let duration = duration_str.map(|s| parse_duration_str(&s)).transpose()?;
    let params = UpdateTimeEntryParams {
        description,
        project_id: project,
        tags,
        billable,
        start,
        stop,
        duration,
    };
    let entry = client
        .update_time_entry(workspace_id, entry_id, &params)
        .await?;
    output::print_success(&mut std::io::stdout(), &entry, json, "Entry updated", hits)
}

async fn delete(
    json: bool,
    workspace_id: WorkspaceId,
    entry_id: TimeEntryId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    client.delete_time_entry(workspace_id, entry_id).await?;
    output::print_deleted(
        &mut std::io::stdout(),
        json,
        &format!("Entry #{entry_id} deleted"),
        hits,
    )
}

async fn continue_entry(
    json: bool,
    entry_id: TimeEntryId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let source = client.get_time_entry(entry_id).await?;
    let params = CreateTimeEntryParams::from(&source);
    let entry = client
        .create_time_entry(source.workspace_id, &params)
        .await?;
    output::print_success(
        &mut std::io::stdout(),
        &entry,
        json,
        "Timer continued",
        hits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::{TimeEntry, TimeEntryId};
    use chrono::Utc;

    fn make_entry(id: i64) -> TimeEntry {
        let now = Utc::now();
        TimeEntry {
            id: TimeEntryId(id),
            workspace_id: WorkspaceId(1),
            description: Some("Test entry".to_string()),
            start: now,
            stop: Some(now),
            duration: 3600,
            project_id: None,
            task_id: None,
            tags: vec!["tag1".to_string()],
            billable: false,
        }
    }

    fn make_running_entry(id: i64) -> TimeEntry {
        let now = Utc::now();
        TimeEntry {
            id: TimeEntryId(id),
            workspace_id: WorkspaceId(1),
            description: Some("Test entry".to_string()),
            start: now,
            stop: None,
            duration: -now.timestamp(),
            project_id: None,
            task_id: None,
            tags: vec![],
            billable: false,
        }
    }

    #[tokio::test]
    async fn list_entries_with_count() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entries()
            .returning(|_, _| Ok(vec![make_entry(1), make_entry(2), make_entry(3)]));
        let result = run(
            EntriesAction::List {
                since: None,
                until: None,
                count: Some(2),
            },
            false,
            None,
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn continue_entry_copies_fields() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entry()
            .withf(|id| *id == TimeEntryId(5))
            .returning(|_| Ok(make_entry(5)));
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_running_entry(6)));
        let result = run(EntriesAction::Continue { id: 5 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_without_count() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entries()
            .returning(|_, _| Ok(vec![make_entry(1), make_entry(2), make_entry(3)]));
        let result = run(
            EntriesAction::List {
                since: None,
                until: None,
                count: None,
            },
            false,
            None,
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_with_since_until() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entries()
            .withf(|s, u| s.as_deref() == Some("2024-01-01") && u.as_deref() == Some("2024-01-31"))
            .returning(|_, _| Ok(vec![make_entry(1)]));
        let result = run(
            EntriesAction::List {
                since: Some("2024-01-01".to_string()),
                until: Some("2024-01-31".to_string()),
                count: None,
            },
            false,
            None,
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_entry_by_id() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entry()
            .withf(|id| *id == TimeEntryId(42))
            .returning(|_| Ok(make_entry(42)));
        let result = run(EntriesAction::Get { id: 42 }, false, None, &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_entry_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entry()
            .returning(|_| Ok(make_entry(42)));
        let result = run(EntriesAction::Get { id: 42 }, true, None, &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edit_entry_updates_fields() {
        let mut mock = MockApiClient::new();
        mock.expect_update_time_entry()
            .withf(|wid, eid, _| *wid == WorkspaceId(1) && *eid == TimeEntryId(10))
            .returning(|_, _, _| Ok(make_entry(10)));
        let result = run(
            EntriesAction::Edit {
                id: 10,
                description: Some("Updated".to_string()),
                project: None,
                tags: None,
                billable: Some(true),
                start: None,
                stop: None,
                duration: None,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edit_entry_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_update_time_entry()
            .returning(|_, _, _| Ok(make_entry(10)));
        let result = run(
            EntriesAction::Edit {
                id: 10,
                description: Some("Updated".to_string()),
                project: None,
                tags: None,
                billable: None,
                start: None,
                stop: None,
                duration: None,
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_entry_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_time_entry()
            .withf(|wid, eid| *wid == WorkspaceId(1) && *eid == TimeEntryId(7))
            .returning(|_, _| Ok(()));
        let result = run(EntriesAction::Delete { id: 7 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn continue_with_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_time_entry()
            .returning(|_| Ok(make_entry(5)));
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_running_entry(6)));
        let result = run(EntriesAction::Continue { id: 5 }, true, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_list_with_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let _guard = crate::ENV_MUTEX.lock().await;
        let server = MockServer::start().await;
        // SAFETY: env var access serialized by ENV_MUTEX
        unsafe { std::env::set_var("TOGGL_API_TOKEN", "test_token") };

        Mock::given(method("GET"))
            .and(path("/me/time_entries"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "workspace_id": 1, "description": "Test",
                    "start": "2024-01-01T00:00:00Z", "stop": "2024-01-01T01:00:00Z",
                    "duration": 3600, "project_id": null, "task_id": null,
                    "tags": [], "billable": false
                }])),
            )
            .mount(&server)
            .await;

        let result = execute_with_base_url(
            EntriesAction::List {
                since: None,
                until: None,
                count: None,
            },
            false,
            Some(1),
            Some(&server.uri()),
        )
        .await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }

    // --- create / edit mock tests ---

    #[tokio::test]
    async fn create_entry_with_stop() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(20)));
        let result = run(
            EntriesAction::Create {
                description: Some("Test".to_string()),
                project: None,
                task: None,
                tags: None,
                billable: false,
                start: "2026-03-20T09:00:00Z".to_string(),
                stop: Some("2026-03-20T10:00:00Z".to_string()),
                duration: None,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_entry_with_duration() {
        let mut mock = MockApiClient::new();
        mock.expect_create_time_entry()
            .returning(|_, _| Ok(make_entry(21)));
        let result = run(
            EntriesAction::Create {
                description: None,
                project: None,
                task: None,
                tags: None,
                billable: false,
                start: "2026-03-20T09:00:00Z".to_string(),
                stop: None,
                duration: Some("1h30m".to_string()),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_entry_rejects_stop_and_duration() {
        let mock = MockApiClient::new();
        let result = run(
            EntriesAction::Create {
                description: None,
                project: None,
                task: None,
                tags: None,
                billable: false,
                start: "2026-03-20T09:00:00Z".to_string(),
                stop: Some("2026-03-20T10:00:00Z".to_string()),
                duration: Some("1h".to_string()),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_entry_rejects_stop_before_start() {
        let mock = MockApiClient::new();
        let result = run(
            EntriesAction::Create {
                description: None,
                project: None,
                task: None,
                tags: None,
                billable: false,
                start: "2026-03-20T10:00:00Z".to_string(),
                stop: Some("2026-03-20T09:00:00Z".to_string()),
                duration: None,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn edit_entry_with_start_time() {
        let mut mock = MockApiClient::new();
        mock.expect_update_time_entry()
            .returning(|_, _, _| Ok(make_entry(10)));
        let result = run(
            EntriesAction::Edit {
                id: 10,
                description: None,
                project: None,
                tags: None,
                billable: None,
                start: Some("2026-03-20T09:30:00Z".to_string()),
                stop: None,
                duration: None,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edit_entry_rejects_stop_and_duration() {
        let mock = MockApiClient::new();
        let result = run(
            EntriesAction::Edit {
                id: 10,
                description: None,
                project: None,
                tags: None,
                billable: None,
                start: None,
                stop: Some("2026-03-20T10:00:00Z".to_string()),
                duration: Some("1h".to_string()),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_err());
    }

    // --- parse_datetime tests ---

    #[test]
    fn parse_datetime_utc_format() {
        let dt = super::parse_datetime("2026-03-20T09:00:00Z").unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-03-20T09:00:00+00:00");
    }

    #[test]
    fn parse_datetime_local_t_separator() {
        let dt = super::parse_datetime("2026-03-20T09:00").unwrap();
        // Should parse without error (exact UTC value depends on local TZ)
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-03-20");
    }

    #[test]
    fn parse_datetime_local_space_separator() {
        let dt = super::parse_datetime("2026-03-20 09:00").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-03-20");
    }

    #[test]
    fn parse_datetime_with_seconds() {
        let dt = super::parse_datetime("2026-03-20T09:30:45").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-03-20");
    }

    #[test]
    fn parse_datetime_invalid_input() {
        assert!(super::parse_datetime("not-a-date").is_err());
        assert!(super::parse_datetime("").is_err());
        assert!(super::parse_datetime("2026-13-01 00:00").is_err());
    }

    // --- parse_duration_str tests ---

    #[test]
    fn parse_duration_str_hours_and_minutes() {
        assert_eq!(super::parse_duration_str("1h30m").unwrap(), 5400);
    }

    #[test]
    fn parse_duration_str_minutes_only() {
        assert_eq!(super::parse_duration_str("90m").unwrap(), 5400);
    }

    #[test]
    fn parse_duration_str_seconds_only() {
        assert_eq!(super::parse_duration_str("30s").unwrap(), 30);
    }

    #[test]
    fn parse_duration_str_pure_number() {
        assert_eq!(super::parse_duration_str("5400").unwrap(), 5400);
    }

    #[test]
    fn parse_duration_str_zero_rejected() {
        assert!(super::parse_duration_str("0m").is_err());
        assert!(super::parse_duration_str("0h").is_err());
    }

    #[test]
    fn parse_duration_str_invalid_input() {
        assert!(super::parse_duration_str("abc").is_err());
        assert!(super::parse_duration_str("1x").is_err());
        assert!(super::parse_duration_str("").is_err());
    }

    #[test]
    fn parse_duration_str_trailing_number_rejected() {
        // "90" without unit should be treated as pure seconds
        assert_eq!(super::parse_duration_str("90").unwrap(), 90);
        // "1h30" has trailing number without unit
        assert!(super::parse_duration_str("1h30").is_err());
    }
}
