use crate::api::client::ApiClient;
use crate::cli::WorkspacesAction;
use crate::commands::{CacheHits, build_client, cached_fetch};
use crate::error::Result;
use crate::models::WorkspaceId;
use crate::output;

pub async fn execute(action: WorkspacesAction, json: bool) -> Result<()> {
    execute_with_base_url(action, json, None).await
}

pub async fn execute_with_base_url(
    action: WorkspacesAction,
    json: bool,
    base_url: Option<&str>,
) -> Result<()> {
    let client = build_client(base_url)?;
    run(action, json, &client).await
}

async fn run(
    action: WorkspacesAction,
    json: bool,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let mut hits = CacheHits::new();
    match action {
        WorkspacesAction::List => list(json, client, &mut hits).await,
        WorkspacesAction::Get { id } => get(json, WorkspaceId(id), client, &hits).await,
    }
}

async fn get(
    json: bool,
    id: WorkspaceId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let ws = client.get_workspace(id).await?;
    output::print_result(&mut std::io::stdout(), &ws, json, hits)
}

async fn list(
    json: bool,
    client: &(impl ApiClient + ?Sized),
    hits: &mut CacheHits,
) -> Result<()> {
    let workspaces = cached_fetch("workspaces", hits, client.list_workspaces()).await?;
    output::print_list(&mut std::io::stdout(), &workspaces, json, hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::{Workspace, WorkspaceId};

    fn make_workspace(id: i64, name: &str) -> Workspace {
        Workspace {
            id: WorkspaceId(id),
            name: name.to_string(),
        }
    }

    #[tokio::test]
    async fn list_workspaces_displays_all() {
        let mut mock = MockApiClient::new();
        mock.expect_list_workspaces()
            .returning(|| Ok(vec![make_workspace(1, "Personal"), make_workspace(2, "Team")]));
        let result = run(WorkspacesAction::List, false, &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_workspaces_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_workspaces()
            .returning(|| Ok(vec![make_workspace(1, "Personal")]));
        let result = run(WorkspacesAction::List, true, &mock).await;
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
            .and(path("/me/workspaces"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "name": "My Workspace"
                }])),
            )
            .mount(&server)
            .await;

        let result =
            execute_with_base_url(WorkspacesAction::List, false, Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
