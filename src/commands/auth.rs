use colored::Colorize;

use crate::api::client::{ApiClient, TogglClient};
use crate::cli::AuthAction;
use crate::constants::ENV_API_TOKEN;
use crate::credentials::{self, CredentialStore, KeyringStore};
use crate::error::{AppError, Result};

pub async fn execute(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { token } => {
            let token = match token {
                Some(t) => {
                    eprintln!("Warning: passing tokens as arguments is visible in process listings and shell history. Consider using interactive input instead.");
                    t
                }
                None => {
                    eprint!("API token: ");
                    rpassword::read_password()
                        .map_err(|e| AppError::Auth(format!("Failed to read token: {e}")))?
                }
            };
            if token.trim().is_empty() {
                return Err(AppError::InvalidInput(
                    "API token cannot be empty".to_string(),
                ));
            }
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

fn warn_if_env_override() {
    if std::env::var(ENV_API_TOKEN)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        eprintln!(
            "Note: environment variable {ENV_API_TOKEN} is set and will take precedence over the keyring."
        );
    }
}

pub async fn login_with_base_url(token: &str, base_url: Option<&str>) -> Result<()> {
    let store = KeyringStore::new()?;
    let client = match base_url {
        Some(url) => TogglClient::new_with_base_url(token, url)?,
        None => TogglClient::new(token)?,
    };
    login_inner(token, &store, &client).await?;
    warn_if_env_override();
    Ok(())
}

fn clear() -> Result<()> {
    let store = KeyringStore::new()?;
    clear_inner(&store)?;
    warn_if_env_override();
    Ok(())
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

        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "email": "t@t.com", "fullname": "T",
                "default_workspace_id": 1, "timezone": "UTC"
            })))
            .mount(&server)
            .await;

        let client = TogglClient::new_with_base_url("test_token", &server.uri()).unwrap();
        let mut store = MockCredentialStore::new();
        store.expect_save().returning(|_| Ok(()));
        let result = login_inner("test_token", &store, &client).await;
        assert!(result.is_ok());
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
