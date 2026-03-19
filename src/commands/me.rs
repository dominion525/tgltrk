use crate::api::client::{ApiClient, TogglClient};
use crate::credentials;
use crate::error::Result;
use crate::output;

pub async fn execute(json: bool, _workspace: Option<i64>) -> Result<()> {
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = TogglClient::new(&cred.api_token)?;
    run(json, &client).await
}

async fn run(json: bool, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    let user = client.get_me().await?;
    output::print_result(&user, json)
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
}
