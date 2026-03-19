use colored::Colorize;

use crate::api::client::{ApiClient, TogglClient};
use crate::cli::AuthAction;
use crate::credentials::{self, CredentialStore};
use crate::error::Result;

pub async fn execute(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { token } => login(&token).await,
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
    let store = credentials::get_store();
    let client = TogglClient::new(token)?;
    login_inner(token, store.as_ref(), &client).await
}

fn clear() -> Result<()> {
    let store = credentials::get_store();
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
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = TogglClient::new(&cred.api_token)?;
    status_inner(&client).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::credentials::MockCredentialStore;
    use crate::models::User;

    fn mock_user() -> User {
        User {
            email: "t@t.com".to_string(),
            fullname: "T".to_string(),
            default_workspace_id: 1,
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
}
