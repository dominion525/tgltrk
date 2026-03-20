use std::collections::HashMap;
use std::io::Write;

use crate::api::client::{ApiClient, CreateProjectParams, UpdateProjectParams};
use crate::cli::ProjectsAction;
use crate::commands::{
    CacheHits, build_client, cached_fetch, invalidate_cache, resolve_workspace_id,
};
use crate::error::Result;
use crate::models::{ClientId, ProjectId, WorkspaceId};
use crate::output;

pub async fn execute(action: ProjectsAction, json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(action, json, workspace, None).await
}

pub async fn execute_with_base_url(
    action: ProjectsAction,
    json: bool,
    workspace: Option<i64>,
    base_url: Option<&str>,
) -> Result<()> {
    let client = build_client(base_url)?;
    run(action, json, workspace, &client).await
}

async fn run(
    action: ProjectsAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let mut hits = CacheHits::new();
    let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
    match action {
        ProjectsAction::List => list(json, wid, client, &mut hits).await,
        ProjectsAction::Get { id } => get(json, wid, ProjectId(id), client, &hits).await,
        ProjectsAction::Create { name, client: cid } => {
            create(json, wid, &name, cid, client, &hits).await
        }
        ProjectsAction::Update {
            id,
            name,
            client: cid,
        } => update(json, wid, ProjectId(id), name, cid, client, &hits).await,
        ProjectsAction::Delete { id } => delete(json, wid, ProjectId(id), client, &hits).await,
    }
}

async fn list(
    json: bool,
    wid: WorkspaceId,
    client: &(impl ApiClient + ?Sized),
    hits: &mut CacheHits,
) -> Result<()> {
    let key = format!("projects_{wid}");
    let projects = cached_fetch(&key, hits, client.list_projects(wid)).await?;
    if json {
        return output::print_list(&mut std::io::stdout(), &projects, json, hits);
    }
    let client_key = format!("clients_{wid}");
    let clients = cached_fetch(&client_key, hits, client.list_clients(wid)).await?;
    let client_map: HashMap<ClientId, String> =
        clients.iter().map(|c| (c.id, c.name.clone())).collect();
    let w = &mut std::io::stdout();
    output::write_cache_hits_text(w, hits)?;
    for p in &projects {
        let client_name = p
            .client_id
            .and_then(|cid| client_map.get(&cid))
            .map(|n| format!(" [{n}]"))
            .unwrap_or_default();
        let status = if p.active { "" } else { " (archived)" };
        writeln!(w, "#{} {}{}{}", p.id, p.name, client_name, status)?;
    }
    Ok(())
}

async fn get(
    json: bool,
    wid: WorkspaceId,
    id: ProjectId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let project = client.get_project(wid, id).await?;
    output::print_result(&mut std::io::stdout(), &project, json, hits)
}

async fn create(
    json: bool,
    wid: WorkspaceId,
    name: &str,
    client_id: Option<i64>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let params = CreateProjectParams {
        name: name.to_string(),
        client_id,
    };
    let project = client.create_project(wid, &params).await?;
    invalidate_cache(&format!("projects_{wid}"));
    output::print_success(
        &mut std::io::stdout(),
        &project,
        json,
        "Project created",
        hits,
    )
}

async fn update(
    json: bool,
    wid: WorkspaceId,
    id: ProjectId,
    name: Option<String>,
    client_id: Option<i64>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let params = UpdateProjectParams { name, client_id };
    let project = client.update_project(wid, id, &params).await?;
    invalidate_cache(&format!("projects_{wid}"));
    output::print_success(
        &mut std::io::stdout(),
        &project,
        json,
        "Project updated",
        hits,
    )
}

async fn delete(
    json: bool,
    wid: WorkspaceId,
    id: ProjectId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    client.delete_project(wid, id).await?;
    invalidate_cache(&format!("projects_{wid}"));
    output::print_deleted(
        &mut std::io::stdout(),
        json,
        &format!("Project #{id} deleted"),
        hits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::Project;

    fn make_project(id: i64, name: &str) -> Project {
        Project {
            id: ProjectId(id),
            workspace_id: WorkspaceId(1),
            name: name.to_string(),
            active: true,
            color: "#06aaf5".to_string(),
            billable: None,
            client_id: None,
        }
    }

    #[tokio::test]
    async fn list_projects_displays_all() {
        let mut mock = MockApiClient::new();
        mock.expect_list_projects().returning(|_| {
            Ok(vec![
                make_project(1, "Project A"),
                make_project(2, "Project B"),
            ])
        });
        mock.expect_list_clients().returning(|_| Ok(vec![]));
        let result = run(ProjectsAction::List, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_project()
            .withf(|wid, pid| *wid == WorkspaceId(1) && *pid == ProjectId(5))
            .returning(|_, _| Ok(()));
        let result = run(ProjectsAction::Delete { id: 5 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_by_id() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .withf(|wid, pid| *wid == WorkspaceId(1) && *pid == ProjectId(10))
            .returning(|_, _| Ok(make_project(10, "My Project")));
        let result = run(ProjectsAction::Get { id: 10 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .returning(|_, _| Ok(make_project(10, "My Project")));
        let result = run(ProjectsAction::Get { id: 10 }, true, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_project()
            .returning(|_, _| Ok(make_project(11, "New")));
        let result = run(
            ProjectsAction::Create {
                name: "New".to_string(),
                client: None,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_project()
            .returning(|_, _| Ok(make_project(11, "New")));
        let result = run(
            ProjectsAction::Create {
                name: "New".to_string(),
                client: None,
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_update_project()
            .withf(|wid, pid, _| *wid == WorkspaceId(1) && *pid == ProjectId(10))
            .returning(|_, _, _| Ok(make_project(10, "Renamed")));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
                client: None,
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_update_project()
            .returning(|_, _, _| Ok(make_project(10, "Renamed")));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
                client: None,
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_projects_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_projects()
            .returning(|_| Ok(vec![make_project(1, "A"), make_project(2, "B")]));
        let result = run(ProjectsAction::List, true, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_list_with_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let server = MockServer::start().await;
        // SAFETY: env var access serialized by ENV_MUTEX
        unsafe { std::env::set_var("TOGGL_API_TOKEN", "test_token") };

        Mock::given(method("GET"))
            .and(path("/workspaces/1/projects"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "workspace_id": 1, "name": "P",
                    "active": true, "color": "#fff", "billable": null
                }])),
            )
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/workspaces/1/clients"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([])),
            )
            .mount(&server)
            .await;

        let result =
            execute_with_base_url(ProjectsAction::List, false, Some(1), Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
