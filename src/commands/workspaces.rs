use crate::api::client::ApiClient;
use crate::cli::WorkspacesAction;
use crate::commands::CommandContext;
use crate::error::Result;
use crate::models::WorkspaceId;
use crate::output;

pub async fn run(
    action: WorkspacesAction,
    ctx: &mut CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    match action {
        WorkspacesAction::List => list(ctx).await,
        WorkspacesAction::Get { id } => get(WorkspaceId(id), ctx).await,
    }
}

async fn list(ctx: &mut CommandContext<'_, impl ApiClient>) -> Result<()> {
    let fut = ctx.client.list_workspaces();
    let workspaces = ctx.cached_fetch("workspaces", fut).await?;
    output::print_list(&mut std::io::stdout(), &workspaces, ctx.json, ctx.hits())
}

async fn get(id: WorkspaceId, ctx: &CommandContext<'_, impl ApiClient>) -> Result<()> {
    let ws = ctx.client.get_workspace(id).await?;
    output::print_result(&mut std::io::stdout(), &ws, ctx.json, ctx.hits())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::commands::build_client;
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
        mock.expect_list_workspaces().returning(|| {
            Ok(vec![
                make_workspace(1, "Personal"),
                make_workspace(2, "Team"),
            ])
        });
        let mut ctx = CommandContext::new(&mock, false, None);
        let result = run(WorkspacesAction::List, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_workspaces_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_workspaces()
            .returning(|| Ok(vec![make_workspace(1, "Personal")]));
        let mut ctx = CommandContext::new(&mock, true, None);
        let result = run(WorkspacesAction::List, &mut ctx).await;
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
            .and(path("/me/workspaces"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "name": "My Workspace"
                }])),
            )
            .mount(&server)
            .await;

        let client = build_client(Some(&server.uri())).unwrap();
        let mut ctx = CommandContext::new(&client, false, None);
        let result = run(WorkspacesAction::List, &mut ctx).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
