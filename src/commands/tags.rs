use colored::Colorize;

use crate::api::client::{ApiClient, TogglClient};
use crate::cli::TagsAction;
use crate::commands::{
    cache_get, cache_set, invalidate_cache, print_cache_hit, resolve_workspace_id,
};
use crate::credentials;
use crate::error::Result;
use crate::output;

pub async fn execute(action: TagsAction, json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(action, json, workspace, None).await
}

pub async fn execute_with_base_url(
    action: TagsAction,
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
    action: TagsAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let wid = resolve_workspace_id(client, workspace).await?;
    match action {
        TagsAction::List => list(json, wid, client).await,
        TagsAction::Create { name } => create(json, wid, &name, client).await,
        TagsAction::Update { id, name } => update(json, wid, id, &name, client).await,
        TagsAction::Delete { id } => delete(wid, id, client).await,
    }
}

async fn list(json: bool, wid: i64, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    if let Some(cached) = cache_get::<Vec<crate::models::Tag>>("tags") {
        if !json {
            print_cache_hit("tags");
        }
        return output::print_list(&cached, json);
    }
    let tags = client.list_tags(wid).await?;
    cache_set("tags", &tags);
    output::print_list(&tags, json)
}

async fn create(
    json: bool,
    wid: i64,
    name: &str,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let tag = client.create_tag(wid, name).await?;
    invalidate_cache("tags");
    if json {
        output::print_result(&tag, true)?;
    } else {
        println!("{} Tag created", "✓".green().bold());
        println!("{tag}");
    }
    Ok(())
}

async fn update(
    json: bool,
    wid: i64,
    id: i64,
    name: &str,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let tag = client.update_tag(wid, id, name).await?;
    invalidate_cache("tags");
    if json {
        output::print_result(&tag, true)?;
    } else {
        println!("{} Tag updated", "✓".green().bold());
        println!("{tag}");
    }
    Ok(())
}

async fn delete(wid: i64, id: i64, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    client.delete_tag(wid, id).await?;
    invalidate_cache("tags");
    println!("{} Tag #{id} deleted", "✓".green().bold());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::Tag;

    fn make_tag(id: i64, name: &str) -> Tag {
        Tag {
            id,
            workspace_id: 1,
            name: name.to_string(),
        }
    }

    #[tokio::test]
    async fn list_tags_displays_all() {
        let mut mock = MockApiClient::new();
        mock.expect_list_tags()
            .returning(|_| Ok(vec![make_tag(1, "urgent"), make_tag(2, "billing")]));
        let result = run(TagsAction::List, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_tag_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_tag()
            .withf(|wid, tid| *wid == 1 && *tid == 3)
            .returning(|_, _| Ok(()));
        let result = run(TagsAction::Delete { id: 3 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_tag_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_tag()
            .returning(|_, _| Ok(make_tag(10, "urgent")));
        let result = run(
            TagsAction::Create {
                name: "urgent".to_string(),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_tag_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_tag()
            .returning(|_, _| Ok(make_tag(10, "urgent")));
        let result = run(
            TagsAction::Create {
                name: "urgent".to_string(),
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_tag_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_update_tag()
            .withf(|wid, tid, _| *wid == 1 && *tid == 5)
            .returning(|_, _, _| Ok(make_tag(5, "renamed")));
        let result = run(
            TagsAction::Update {
                id: 5,
                name: "renamed".to_string(),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_tag_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_update_tag()
            .returning(|_, _, _| Ok(make_tag(5, "renamed")));
        let result = run(
            TagsAction::Update {
                id: 5,
                name: "renamed".to_string(),
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_tags_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_tags()
            .returning(|_| Ok(vec![make_tag(1, "urgent"), make_tag(2, "billing")]));
        let result = run(TagsAction::List, true, Some(1), &mock).await;
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
            .and(path("/workspaces/1/tags"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                    "id": 1, "workspace_id": 1, "name": "urgent"
                }])),
            )
            .mount(&server)
            .await;

        let result =
            execute_with_base_url(TagsAction::List, false, Some(1), Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
