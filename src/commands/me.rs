use crate::api::client::ApiClient;
use crate::commands::CommandContext;
use crate::error::Result;
use crate::output;

pub async fn run(ctx: &mut CommandContext<'_, impl ApiClient>) -> Result<()> {
    let fut = ctx.client.get_me();
    let user = ctx.cached_fetch("user", fut).await?;
    output::print_result(&mut std::io::stdout(), &user, ctx.json, ctx.hits())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::commands::build_client;
    use crate::models::{User, WorkspaceId};

    #[tokio::test]
    async fn me_displays_user() {
        let mut mock = MockApiClient::new();
        mock.expect_get_me().returning(|| {
            Ok(User {
                email: "test@example.com".to_string(),
                fullname: "Test User".to_string(),
                default_workspace_id: WorkspaceId(123),
                timezone: "Asia/Tokyo".to_string(),
            })
        });
        let mut ctx = CommandContext::new(&mock, false, None);
        let result = run(&mut ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_with_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let _guard = crate::ENV_MUTEX.lock().await;
        let server = MockServer::start().await;
        // SAFETY: env var access serialized by ENV_MUTEX
        unsafe { std::env::set_var("TOGGL_API_TOKEN", "test_token") };

        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "email": "t@t.com", "fullname": "T",
                "default_workspace_id": 1, "timezone": "UTC"
            })))
            .mount(&server)
            .await;

        let client = build_client(Some(&server.uri())).unwrap();
        let mut ctx = CommandContext::new(&client, false, None);
        let result = run(&mut ctx).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
