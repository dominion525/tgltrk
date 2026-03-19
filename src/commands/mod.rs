pub mod auth;
pub mod cache_cmd;
pub mod entries;
pub mod me;
pub mod projects;
pub mod tags;
pub mod timer;

use chrono::TimeDelta;
use colored::Colorize;

use crate::api::client::ApiClient;
use crate::cache::FileCache;
use crate::error::Result;
use crate::models::User;

fn get_cache() -> Option<FileCache> {
    FileCache::default_with_ttl(TimeDelta::hours(72)).ok()
}

pub async fn resolve_workspace_id(
    client: &(impl ApiClient + ?Sized),
    workspace_override: Option<i64>,
) -> Result<i64> {
    resolve_workspace_id_inner(client, workspace_override, &get_cache()).await
}

async fn resolve_workspace_id_inner(
    client: &(impl ApiClient + ?Sized),
    workspace_override: Option<i64>,
    cache: &Option<FileCache>,
) -> Result<i64> {
    if let Some(id) = workspace_override {
        return Ok(id);
    }

    // Try cache first
    if let Some(cache) = cache {
        if let Some(user) = cache.get::<User>("user") {
            print_cache_hit("user");
            return Ok(user.default_workspace_id);
        }
    }

    let user = client.get_me().await?;

    // Cache the result
    if let Some(cache) = cache {
        let _ = cache.set("user", &user);
    }

    Ok(user.default_workspace_id)
}

pub fn invalidate_cache(key: &str) {
    if let Some(cache) = get_cache() {
        let _ = cache.invalidate(key);
    }
}

pub fn cache_set<T: serde::Serialize>(key: &str, value: &T) {
    if let Some(cache) = get_cache() {
        let _ = cache.set(key, value);
    }
}

pub fn cache_get<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    get_cache().and_then(|cache| cache.get(key))
}

pub fn print_cache_hit(entity: &str) {
    println!("{}", format!("(cached: {entity})").dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;

    #[tokio::test]
    async fn resolve_workspace_id_uses_override() {
        let mock = MockApiClient::new();
        let result = resolve_workspace_id_inner(&mock, Some(42), &None)
            .await
            .unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn resolve_workspace_id_falls_back_to_api() {
        let mut mock = MockApiClient::new();
        mock.expect_get_me().returning(|| {
            Ok(User {
                email: "test@example.com".to_string(),
                fullname: "Test".to_string(),
                default_workspace_id: 99,
                timezone: "UTC".to_string(),
            })
        });
        let result = resolve_workspace_id_inner(&mock, None, &None)
            .await
            .unwrap();
        assert_eq!(result, 99);
    }
}
