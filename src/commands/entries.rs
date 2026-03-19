use crate::api::client::{ApiClient, CreateTimeEntryParams, TogglClient, UpdateTimeEntryParams};
use crate::cli::EntriesAction;
use crate::commands::{resolve_workspace_id, CacheHits};
use crate::credentials;
use crate::error::Result;
use crate::output;

pub async fn execute(action: EntriesAction, json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(action, json, workspace, None).await
}

pub async fn execute_with_base_url(
    action: EntriesAction,
    json: bool,
    workspace: Option<i64>,
    base_url: Option<&str>,
) -> Result<()> {
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = match base_url {
        Some(url) => TogglClient::new_with_base_url(&cred.api_token, url)?,
        None => TogglClient::new(&cred.api_token)?,
    };
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
        EntriesAction::Get { id } => get(json, id, client, &hits).await,
        EntriesAction::Edit {
            id,
            description,
            project,
            tags,
            billable,
        } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            edit(json, wid, id, description, project, tags, billable, client, &hits).await
        }
        EntriesAction::Delete { id } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            delete(json, wid, id, client, &hits).await
        }
        EntriesAction::Continue { id } => {
            let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
            continue_entry(json, wid, id, client, &hits).await
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
    if let Some(n) = count {
        entries.truncate(n);
    }
    output::print_list(&entries, json, hits)
}

async fn get(
    json: bool,
    id: i64,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let entry = client.get_time_entry(id).await?;
    output::print_result(&entry, json, hits)
}

#[allow(clippy::too_many_arguments)]
async fn edit(
    json: bool,
    workspace_id: i64,
    entry_id: i64,
    description: Option<String>,
    project: Option<i64>,
    tags: Option<Vec<String>>,
    billable: Option<bool>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let params = UpdateTimeEntryParams {
        description,
        project_id: project,
        tags,
        billable,
    };
    let entry = client
        .update_time_entry(workspace_id, entry_id, &params)
        .await?;
    output::print_success(&entry, json, "Entry updated", hits)
}

async fn delete(
    json: bool,
    workspace_id: i64,
    entry_id: i64,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    client.delete_time_entry(workspace_id, entry_id).await?;
    output::print_deleted(json, &format!("Entry #{entry_id} deleted"), hits)
}

async fn continue_entry(
    json: bool,
    workspace_id: i64,
    entry_id: i64,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let source = client.get_time_entry(entry_id).await?;
    let params = CreateTimeEntryParams {
        description: source.description,
        project_id: source.project_id,
        task_id: source.task_id,
        tags: source.tags,
        billable: source.billable,
    };
    let entry = client.create_time_entry(workspace_id, &params).await?;
    output::print_success(&entry, json, "Timer continued", hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::TimeEntry;
    use chrono::Utc;

    fn make_entry(id: i64) -> TimeEntry {
        let now = Utc::now();
        TimeEntry {
            id,
            workspace_id: 1,
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
            id,
            workspace_id: 1,
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
            .withf(|id| *id == 5)
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
            .withf(|id| *id == 42)
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
            .withf(|wid, eid, _| *wid == 1 && *eid == 10)
            .returning(|_, _, _| Ok(make_entry(10)));
        let result = run(
            EntriesAction::Edit {
                id: 10,
                description: Some("Updated".to_string()),
                project: None,
                tags: None,
                billable: Some(true),
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
            .withf(|wid, eid| *wid == 1 && *eid == 7)
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
}
