use crate::api::client::{ApiClient, CreateProjectParams, UpdateProjectParams};
use crate::cli::ProjectsAction;
use crate::commands::{
    CacheHits, build_client, cached_fetch, invalidate_cache, resolve_workspace_id,
};
use crate::error::Result;
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
        ProjectsAction::Get { id } => get(json, wid, id, client, &hits).await,
        ProjectsAction::Create { name } => create(json, wid, &name, client, &hits).await,
        ProjectsAction::Update { id, name } => update(json, wid, id, name, client, &hits).await,
        ProjectsAction::Delete { id } => delete(json, wid, id, client, &hits).await,
    }
}

async fn list(
    json: bool,
    wid: i64,
    client: &(impl ApiClient + ?Sized),
    hits: &mut CacheHits,
) -> Result<()> {
    let projects = cached_fetch("projects", hits, client.list_projects(wid)).await?;
    output::print_list(&projects, json, hits)
}

async fn get(
    json: bool,
    wid: i64,
    id: i64,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let project = client.get_project(wid, id).await?;
    output::print_result(&project, json, hits)
}

async fn create(
    json: bool,
    wid: i64,
    name: &str,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let params = CreateProjectParams {
        name: name.to_string(),
    };
    let project = client.create_project(wid, &params).await?;
    invalidate_cache("projects");
    output::print_success(&project, json, "Project created", hits)
}

async fn update(
    json: bool,
    wid: i64,
    id: i64,
    name: Option<String>,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let params = UpdateProjectParams { name };
    let project = client.update_project(wid, id, &params).await?;
    invalidate_cache("projects");
    output::print_success(&project, json, "Project updated", hits)
}

async fn delete(
    json: bool,
    wid: i64,
    id: i64,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    client.delete_project(wid, id).await?;
    invalidate_cache("projects");
    output::print_deleted(json, &format!("Project #{id} deleted"), hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::Project;

    fn make_project(id: i64, name: &str) -> Project {
        Project {
            id,
            workspace_id: 1,
            name: name.to_string(),
            active: true,
            color: "#06aaf5".to_string(),
            billable: None,
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
        let result = run(ProjectsAction::List, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_project()
            .withf(|wid, pid| *wid == 1 && *pid == 5)
            .returning(|_, _| Ok(()));
        let result = run(ProjectsAction::Delete { id: 5 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_by_id() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .withf(|wid, pid| *wid == 1 && *pid == 10)
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
            .withf(|wid, pid, _| *wid == 1 && *pid == 10)
            .returning(|_, _, _| Ok(make_project(10, "Renamed")));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
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

        let _guard = crate::ENV_MUTEX.lock().unwrap();
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

        let result =
            execute_with_base_url(ProjectsAction::List, false, Some(1), Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
