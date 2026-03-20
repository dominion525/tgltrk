use colored::Colorize;

use crate::api::client::{ApiClient, TogglClient};
use crate::cli::AuthAction;
use crate::credentials::{self, CredentialStore};
use crate::error::{AppError, Result};

pub async fn execute(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { token } => {
            let token = match token {
                Some(t) => t,
                None => {
                    eprint!("API token: ");
                    rpassword::read_password()
                        .map_err(|e| AppError::Auth(format!("Failed to read token: {e}")))?
                }
            };
            login(&token).await
        }
        AuthAction::Clear => clear(),
        AuthAction::Status => status().await,
    }
}

async fn login_inner(
    token: &str,
    store: &dyn CredentialStore,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let user = client.get_me().await?;
    store.save(token)?;
    super::clear_all_cache();
    println!(
        "{} Authenticated as {} ({})",
        "✓".green().bold(),
        user.fullname,
        user.email
    );
    Ok(())
}

fn clear_inner(store: &dyn CredentialStore) -> Result<()> {
    store.clear()?;
    println!("{} Token removed", "✓".green().bold());
    Ok(())
}

async fn login(token: &str) -> Result<()> {
    login_with_base_url(token, None).await
}

pub async fn login_with_base_url(token: &str, base_url: Option<&str>) -> Result<()> {
    let store = credentials::get_store()?;
    let client = match base_url {
        Some(url) => TogglClient::new_with_base_url(token, url)?,
        None => TogglClient::new(token)?,
    };
    login_inner(token, store.as_ref(), &client).await
}

fn clear() -> Result<()> {
    let store = credentials::get_store()?;
    clear_inner(store.as_ref())
}

async fn status_inner(client: &(impl ApiClient + ?Sized)) -> Result<()> {
    let user = client.get_me().await?;
    println!(
        "{} Authenticated as {} ({})",
        "✓".green().bold(),
        user.fullname,
        user.email
    );
    Ok(())
}

async fn status() -> Result<()> {
    status_with_base_url(None).await
}

pub async fn status_with_base_url(base_url: Option<&str>) -> Result<()> {
    let store = credentials::get_store()?;
    let cred = store.read()?;
    let client = match base_url {
        Some(url) => TogglClient::new_with_base_url(&cred.api_token, url)?,
        None => TogglClient::new(&cred.api_token)?,
    };
    status_inner(&client).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::credentials::MockCredentialStore;
    use crate::models::{User, WorkspaceId};

    fn mock_user() -> User {
        User {
            email: "t@t.com".to_string(),
            fullname: "T".to_string(),
            default_workspace_id: WorkspaceId(1),
            timezone: "UTC".to_string(),
        }
    }

    #[test]
    fn clear_inner_calls_store_clear() {
        let mut store = MockCredentialStore::new();
        store.expect_clear().returning(|| Ok(()));
        assert!(clear_inner(&store).is_ok());
    }

    #[tokio::test]
    async fn login_inner_validates_and_saves() {
        let mut mock = MockApiClient::new();
        mock.expect_get_me().returning(|| Ok(mock_user()));

        let mut store = MockCredentialStore::new();
        store.expect_save().returning(|_| Ok(()));

        let result = login_inner("test_token", &store, &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn status_inner_shows_user() {
        let mut mock = MockApiClient::new();
        mock.expect_get_me().returning(|| Ok(mock_user()));
        let result = status_inner(&mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn login_with_wiremock() {
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

        let result = login_with_base_url("test_token", Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        // EnvStore returns error on save, so this will fail at store.save()
        // That's expected behavior - the login validates the token but can't save to env store
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn status_with_wiremock() {
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

        let result = status_with_base_url(Some(&server.uri())).await;
        // SAFETY: test is single-threaded for env var access
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
        assert!(result.is_ok());
    }
}
