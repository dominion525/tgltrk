use std::collections::HashMap;
use std::io::Write;

use crate::api::client::{ApiClient, CreateProjectParams, UpdateProjectParams};
use crate::cli::ProjectsAction;
use crate::commands::CommandContext;
use crate::error::Result;
use crate::models::{ClientId, ProjectId, WorkspaceId};
use crate::output;

pub async fn run(
    action: ProjectsAction,
    ctx: &mut CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let wid = ctx.resolve_workspace_id().await?;
    match action {
        ProjectsAction::List => list(wid, ctx).await,
        ProjectsAction::Get { id } => get(wid, ProjectId(id), ctx).await,
        ProjectsAction::Create { name, client: cid } => {
            create(wid, &name, cid, ctx).await
        }
        ProjectsAction::Update {
            id,
            name,
            client: cid,
        } => update(wid, ProjectId(id), name, cid, ctx).await,
        ProjectsAction::Delete { id } => delete(wid, ProjectId(id), ctx).await,
    }
}

async fn list(
    wid: WorkspaceId,
    ctx: &mut CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let key = format!("projects_{wid}");
    let fut = ctx.client.list_projects(wid);
    let projects = ctx.cached_fetch(&key, fut).await?;
    if ctx.json {
        return output::print_list(&mut std::io::stdout(), &projects, ctx.json, ctx.hits());
    }
    let client_key = format!("clients_{wid}");
    let fut = ctx.client.list_clients(wid);
    let clients = match ctx.cached_fetch(&client_key, fut).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: failed to fetch clients: {e}");
            vec![]
        }
    };
    let client_map: HashMap<ClientId, String> =
        clients.iter().map(|c| (c.id, c.name.clone())).collect();
    let w = &mut std::io::stdout();
    output::write_cache_hits_text(w, ctx.hits())?;
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
    wid: WorkspaceId,
    id: ProjectId,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let project = ctx.client.get_project(wid, id).await?;
    output::print_result(&mut std::io::stdout(), &project, ctx.json, ctx.hits())
}

async fn create(
    wid: WorkspaceId,
    name: &str,
    client_id: Option<i64>,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let params = CreateProjectParams {
        name: name.to_string(),
        client_id,
    };
    let project = ctx.client.create_project(wid, &params).await?;
    ctx.invalidate_cache(&format!("projects_{wid}"));
    output::print_success(
        &mut std::io::stdout(),
        &project,
        ctx.json,
        "Project created",
        ctx.hits(),
    )
}

async fn update(
    wid: WorkspaceId,
    id: ProjectId,
    name: Option<String>,
    client_id: Option<i64>,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let params = UpdateProjectParams { name, client_id };
    let project = ctx.client.update_project(wid, id, &params).await?;
    ctx.invalidate_cache(&format!("projects_{wid}"));
    output::print_success(
        &mut std::io::stdout(),
        &project,
        ctx.json,
        "Project updated",
        ctx.hits(),
    )
}

async fn delete(
    wid: WorkspaceId,
    id: ProjectId,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    ctx.client.delete_project(wid, id).await?;
    ctx.invalidate_cache(&format!("projects_{wid}"));
    output::print_deleted(
        &mut std::io::stdout(),
        ctx.json,
        &format!("Project #{id} deleted"),
        ctx.hits(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::commands::build_client;
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
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(ProjectsAction::List, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_project()
            .withf(|wid, pid| *wid == WorkspaceId(1) && *pid == ProjectId(5))
            .returning(|_, _| Ok(()));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(ProjectsAction::Delete { id: 5 }, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_by_id() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .withf(|wid, pid| *wid == WorkspaceId(1) && *pid == ProjectId(10))
            .returning(|_, _| Ok(make_project(10, "My Project")));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(ProjectsAction::Get { id: 10 }, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .returning(|_, _| Ok(make_project(10, "My Project")));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(ProjectsAction::Get { id: 10 }, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_project()
            .returning(|_, _| Ok(make_project(11, "New")));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            ProjectsAction::Create {
                name: "New".to_string(),
                client: None,
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_project()
            .returning(|_, _| Ok(make_project(11, "New")));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(
            ProjectsAction::Create {
                name: "New".to_string(),
                client: None,
            },
            &mut ctx,
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
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
                client: None,
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_update_project()
            .returning(|_, _, _| Ok(make_project(10, "Renamed")));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
                client: None,
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_projects_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_projects()
            .returning(|_| Ok(vec![make_project(1, "A"), make_project(2, "B")]));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(ProjectsAction::List, &mut ctx).await;
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

        let client = build_client(Some(&server.uri())).unwrap();
        let mut ctx = CommandContext::new(&client, false, Some(1));
        let result = run(ProjectsAction::List, &mut ctx).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
