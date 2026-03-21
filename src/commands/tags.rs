use crate::api::client::ApiClient;
use crate::cli::TagsAction;
use crate::commands::CommandContext;
use crate::error::Result;
use crate::models::{TagId, WorkspaceId};
use crate::output;

pub async fn run(
    action: TagsAction,
    ctx: &mut CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let wid = ctx.resolve_workspace_id().await?;
    match action {
        TagsAction::List => list(wid, ctx).await,
        TagsAction::Create { name } => create(wid, &name, ctx).await,
        TagsAction::Update { id, name } => update(wid, TagId(id), &name, ctx).await,
        TagsAction::Delete { id } => delete(wid, TagId(id), ctx).await,
    }
}

async fn list(
    wid: WorkspaceId,
    ctx: &mut CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let key = format!("tags_{wid}");
    let fut = ctx.client.list_tags(wid);
    let tags = ctx.cached_fetch(&key, fut).await?;
    output::print_list(&mut std::io::stdout(), &tags, ctx.json, ctx.hits())
}

async fn create(
    wid: WorkspaceId,
    name: &str,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let tag = ctx.client.create_tag(wid, name).await?;
    ctx.invalidate_cache(&format!("tags_{wid}"));
    output::print_success(&mut std::io::stdout(), &tag, ctx.json, "Tag created", ctx.hits())
}

async fn update(
    wid: WorkspaceId,
    id: TagId,
    name: &str,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    let tag = ctx.client.update_tag(wid, id, name).await?;
    ctx.invalidate_cache(&format!("tags_{wid}"));
    output::print_success(&mut std::io::stdout(), &tag, ctx.json, "Tag updated", ctx.hits())
}

async fn delete(
    wid: WorkspaceId,
    id: TagId,
    ctx: &CommandContext<'_, impl ApiClient>,
) -> Result<()> {
    ctx.client.delete_tag(wid, id).await?;
    ctx.invalidate_cache(&format!("tags_{wid}"));
    output::print_deleted(
        &mut std::io::stdout(),
        ctx.json,
        &format!("Tag #{id} deleted"),
        ctx.hits(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::commands::build_client;
    use crate::models::Tag;

    fn make_tag(id: i64, name: &str) -> Tag {
        Tag {
            id: TagId(id),
            workspace_id: WorkspaceId(1),
            name: name.to_string(),
        }
    }

    #[tokio::test]
    async fn list_tags_displays_all() {
        let mut mock = MockApiClient::new();
        mock.expect_list_tags()
            .returning(|_| Ok(vec![make_tag(1, "urgent"), make_tag(2, "billing")]));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(TagsAction::List, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_tag_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_tag()
            .withf(|wid, tid| *wid == WorkspaceId(1) && *tid == TagId(3))
            .returning(|_, _| Ok(()));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(TagsAction::Delete { id: 3 }, &mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_tag_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_tag()
            .returning(|_, _| Ok(make_tag(10, "urgent")));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            TagsAction::Create {
                name: "urgent".to_string(),
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_tag_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_tag()
            .returning(|_, _| Ok(make_tag(10, "urgent")));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(
            TagsAction::Create {
                name: "urgent".to_string(),
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_tag_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_update_tag()
            .withf(|wid, tid, _| *wid == WorkspaceId(1) && *tid == TagId(5))
            .returning(|_, _, _| Ok(make_tag(5, "renamed")));
        let mut ctx = CommandContext::new(&mock, false, Some(1));
        let result = run(
            TagsAction::Update {
                id: 5,
                name: "renamed".to_string(),
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_tag_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_update_tag()
            .returning(|_, _, _| Ok(make_tag(5, "renamed")));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(
            TagsAction::Update {
                id: 5,
                name: "renamed".to_string(),
            },
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_tags_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_tags()
            .returning(|_| Ok(vec![make_tag(1, "urgent"), make_tag(2, "billing")]));
        let mut ctx = CommandContext::new(&mock, true, Some(1));
        let result = run(TagsAction::List, &mut ctx).await;
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

        let client = build_client(Some(&server.uri())).unwrap();
        let mut ctx = CommandContext::new(&client, false, Some(1));
        let result = run(TagsAction::List, &mut ctx).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
