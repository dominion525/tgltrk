use crate::api::client::{ApiClient, TogglClient};
use crate::commands::{cache_get, cache_set, CacheHits};
use crate::credentials;
use crate::error::Result;
use crate::models::User;
use crate::output;

pub async fn execute(json: bool, workspace: Option<i64>) -> Result<()> {
    execute_with_base_url(json, workspace, None).await
}

pub async fn execute_with_base_url(
    json: bool,
    _workspace: Option<i64>,
    base_url: Option<&str>,
) -> Result<()> {
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = match base_url {
        Some(url) => TogglClient::new_with_base_url(&cred.api_token, url)?,
        None => TogglClient::new(&cred.api_token)?,
    };
    run(json, &client).await
}

async fn run(json: bool, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    let mut hits = CacheHits::new();
    if let Some(cached) = cache_get::<User>("user") {
        hits.record("user");
        return output::print_result(&cached, json, &hits);
    }
    let user = client.get_me().await?;
    cache_set("user", &user);
    output::print_result(&user, json, &hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::User;

    #[tokio::test]
    async fn me_displays_user() {
        let mut mock = MockApiClient::new();
        mock.expect_get_me().returning(|| {
            Ok(User {
                email: "test@example.com".to_string(),
                fullname: "Test User".to_string(),
                default_workspace_id: 123,
                timezone: "Asia/Tokyo".to_string(),
            })
        });
        let result = run(false, &mock).await;
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

        let result = execute_with_base_url(false, None, Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
