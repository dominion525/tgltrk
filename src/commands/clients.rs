use crate::api::client::ApiClient;
use crate::cli::ClientsAction;
use crate::commands::CommandContext;
use crate::error::Result;
use crate::models::{ClientId, WorkspaceId};
use crate::output;

pub async fn run(
    action: ClientsAction,
    ctx: &mut CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let wid = ctx.resolve_workspace_id().await?;
    match action {
        ClientsAction::List => list(wid, ctx).await,
        ClientsAction::Get { id } => get(wid, ClientId(id), ctx).await,
        ClientsAction::Create { name } => create(wid, &name, ctx).await,
        ClientsAction::Update { id, name } => update(wid, ClientId(id), &name, ctx).await,
        ClientsAction::Delete { id } => delete(wid, ClientId(id), ctx).await,
    }
}

async fn list(wid: WorkspaceId, ctx: &mut CommandContext<'_, impl ApiClient>) -> Result<()> {
    let key = format!("clients_{wid}");
    let fut = ctx.client.list_clients(wid);
    let clients = ctx.cached_fetch(&key, fut).await?;
    output::print_list(&mut std::io::stdout(), &clients, ctx.json, ctx.hits())
}

async fn get(
    wid: WorkspaceId,
    id: ClientId,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let c = ctx.client.get_client(wid, id).await?;
    output::print_result(&mut std::io::stdout(), &c, ctx.json, ctx.hits())
}

async fn create(
    wid: WorkspaceId,
    name: &str,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let c = ctx.client.create_client(wid, name).await?;
    ctx.invalidate_cache(&format!("clients_{wid}"));
    output::print_success(
        &mut std::io::stdout(),
        &c,
        ctx.json,
        "Client created",
        ctx.hits(),
    )
}

async fn update(
    wid: WorkspaceId,
    id: ClientId,
    name: &str,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let c = ctx.client.update_client(wid, id, name).await?;
    ctx.invalidate_cache(&format!("clients_{wid}"));
    output::print_success(
        &mut std::io::stdout(),
        &c,
        ctx.json,
        "Client updated",
        ctx.hits(),
    )
}

async fn delete(
    wid: WorkspaceId,
    id: ClientId,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    ctx.client.delete_client(wid, id).await?;
    ctx.invalidate_cache(&format!("clients_{wid}"));
    output::print_deleted(
        &mut std::io::stdout(),
        ctx.json,
        &format!("Client #{id} deleted"),
        ctx.hits(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::commands::build_client;
    use crate::models::Client;

    fn make_client(id: i64, name: &str) -> Client {
        Client {
            id: ClientId(id),
            workspace_id: WorkspaceId(1),
            name: name.to_string(),
        }
    }

    #[tokio::test]
    async fn list_clients_displays_all() {
        let mut mock = MockApiClient::new();
        mock.expect_list_clients()
            .returning(|_| Ok(vec![make_client(1, "Acme"), make_client(2, "Globex")]));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(ClientsAction::List, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_clients_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_clients()
            .returning(|_| Ok(vec![make_client(1, "Acme")]));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(ClientsAction::List, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_client_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_client()
            .returning(|_, _| Ok(make_client(10, "NewCo")));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            ClientsAction::Create {
                name: "NewCo".to_string(),
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_client_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_update_client()
            .withf(|wid, cid, _| *wid == WorkspaceId(1) && *cid == ClientId(5))
            .returning(|_, _, _| Ok(make_client(5, "Renamed")));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            ClientsAction::Update {
                id: 5,
                name: "Renamed".to_string(),
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_client_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_client()
            .withf(|wid, cid| *wid == WorkspaceId(1) && *cid == ClientId(3))
            .returning(|_, _| Ok(()));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(ClientsAction::Delete { id: 3 }, &mut ctx).await;
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
            .and(path("/workspaces/1/clients"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "wid": 1, "name": "Acme"
                }])),
            )
            .mount(&server)
            .await;

        let client = build_client(Some(&server.uri())).unwrap();
        let mut ctx = CommandContext::new(&client, false, Some(1));
        let result = run(ClientsAction::List, &mut ctx).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
