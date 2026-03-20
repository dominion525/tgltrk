use crate::api::client::ApiClient;
use crate::cli::ClientsAction;
use crate::commands::{
    CacheHits, build_client, cached_fetch, invalidate_cache, resolve_workspace_id,
};
use crate::error::Result;
use crate::models::{ClientId, WorkspaceId};
use crate::output;

pub async fn execute(action: ClientsAction, json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(action, json, workspace, None).await
}

pub async fn execute_with_base_url(
    action: ClientsAction,
    json: bool,
    workspace: Option<i64>,
    base_url: Option<&str>,
) -> Result<()> {
    let client = build_client(base_url)?;
    run(action, json, workspace, &client).await
}

async fn run(
    action: ClientsAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let mut hits = CacheHits::new();
    let wid = resolve_workspace_id(client, workspace, &mut hits).await?;
    match action {
        ClientsAction::List => list(json, wid, client, &mut hits).await,
        ClientsAction::Get { id } => get(json, wid, ClientId(id), client, &hits).await,
        ClientsAction::Create { name } => create(json, wid, &name, client, &hits).await,
        ClientsAction::Update { id, name } => {
            update(json, wid, ClientId(id), &name, client, &hits).await
        }
        ClientsAction::Delete { id } => delete(json, wid, ClientId(id), client, &hits).await,
    }
}

async fn list(
    json: bool,
    wid: WorkspaceId,
    client: &(impl ApiClient + ?Sized),
    hits: &mut CacheHits,
) -> Result<()> {
    let key = format!("clients_{wid}");
    let clients = cached_fetch(&key, hits, client.list_clients(wid)).await?;
    output::print_list(&mut std::io::stdout(), &clients, json, hits)
}

async fn get(
    json: bool,
    wid: WorkspaceId,
    id: ClientId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let c = client.get_client(wid, id).await?;
    output::print_result(&mut std::io::stdout(), &c, json, hits)
}

async fn create(
    json: bool,
    wid: WorkspaceId,
    name: &str,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let c = client.create_client(wid, name).await?;
    invalidate_cache(&format!("clients_{wid}"));
    output::print_success(&mut std::io::stdout(), &c, json, "Client created", hits)
}

async fn update(
    json: bool,
    wid: WorkspaceId,
    id: ClientId,
    name: &str,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    let c = client.update_client(wid, id, name).await?;
    invalidate_cache(&format!("clients_{wid}"));
    output::print_success(&mut std::io::stdout(), &c, json, "Client updated", hits)
}

async fn delete(
    json: bool,
    wid: WorkspaceId,
    id: ClientId,
    client: &(impl ApiClient + ?Sized),
    hits: &CacheHits,
) -> Result<()> {
    client.delete_client(wid, id).await?;
    invalidate_cache(&format!("clients_{wid}"));
    output::print_deleted(
        &mut std::io::stdout(),
        json,
        &format!("Client #{id} deleted"),
        hits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
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
        let result = run(ClientsAction::List, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_clients_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_clients()
            .returning(|_| Ok(vec![make_client(1, "Acme")]));
        let result = run(ClientsAction::List, true, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_client_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_client()
            .returning(|_, _| Ok(make_client(10, "NewCo")));
        let result = run(
            ClientsAction::Create {
                name: "NewCo".to_string(),
            },
            false,
            Some(1),
            &mock,
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
        let result = run(
            ClientsAction::Update {
                id: 5,
                name: "Renamed".to_string(),
            },
            false,
            Some(1),
            &mock,
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
        let result = run(ClientsAction::Delete { id: 3 }, false, Some(1), &mock).await;
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
            .and(path("/workspaces/1/clients"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "wid": 1, "name": "Acme"
                }])),
            )
            .mount(&server)
            .await;

        let result = execute_with_base_url(ClientsAction::List, false, Some(1), Some(&server.uri()))
            .await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
