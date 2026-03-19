use colored::Colorize;

use crate::api::client::{ApiClient, TogglClient};
use crate::cli::AuthAction;
use crate::credentials;
use crate::error::Result;

pub async fn execute(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { token } => login(&token).await,
        AuthAction::Clear => clear(),
        AuthAction::Status => status().await,
    }
}

async fn login(token: &str) -> Result<()> {
    let store = credentials::get_store();

    // Validate token by calling the API
    let client = TogglClient::new(token)?;
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

fn clear() -> Result<()> {
    let store = credentials::get_store();
    store.clear()?;
    println!("{} Token removed", "✓".green().bold());
    Ok(())
}

async fn status() -> Result<()> {
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = TogglClient::new(&cred.api_token)?;
    let user = client.get_me().await?;
    println!(
        "{} Authenticated as {} ({})",
        "✓".green().bold(),
        user.fullname,
        user.email
    );
    Ok(())
}
