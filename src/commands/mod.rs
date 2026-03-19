pub mod auth;
pub mod cache_cmd;
pub mod entries;
pub mod me;
pub mod projects;
pub mod tags;
pub mod timer;

use chrono::TimeDelta;

use crate::api::client::ApiClient;
use crate::cache::FileCache;
use crate::error::Result;
use crate::models::User;

/// キャッシュヒットしたエンティティ名を収集する
#[derive(Default, Debug)]
pub struct CacheHits(Vec<String>);

impl CacheHits {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn record(&mut self, entity: &str) {
        self.0.push(entity.to_string());
    }

    pub fn entities(&self) -> &[String] {
        &self.0
    }
}

fn get_cache() -> Option<FileCache> {
    FileCache::default_with_ttl(TimeDelta::hours(72)).ok()
}

pub async fn resolve_workspace_id(
    client: &(impl ApiClient + ?Sized),
    workspace_override: Option<i64>,
    hits: &mut CacheHits,
) -> Result<i64> {
    resolve_workspace_id_inner(client, workspace_override, &get_cache(), hits).await
}

async fn resolve_workspace_id_inner(
    client: &(impl ApiClient + ?Sized),
    workspace_override: Option<i64>,
    cache: &Option<FileCache>,
    hits: &mut CacheHits,
) -> Result<i64> {
    if let Some(id) = workspace_override {
        return Ok(id);
    }

    // Try cache first
    if let Some(cache) = cache {
        if let Some(user) = cache.get::<User>("user") {
            hits.record("user");
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

pub async fn cached_fetch<T, Fut>(
    key: &str,
    hits: &mut CacheHits,
    fetch: Fut,
) -> Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    Fut: std::future::Future<Output = Result<T>>,
{
    let cache = get_cache();
    if let Some(cached) = cache.as_ref().and_then(|c| c.get::<T>(key)) {
        hits.record(key);
        return Ok(cached);
    }
    let value = fetch.await?;
    if let Some(c) = &cache {
        let _ = c.set(key, &value);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;

    #[tokio::test]
    async fn resolve_workspace_id_uses_override() {
        let mock = MockApiClient::new();
        let mut hits = CacheHits::new();
        let result = resolve_workspace_id_inner(&mock, Some(42), &None, &mut hits)
            .await
            .unwrap();
        assert_eq!(result, 42);
        assert!(hits.entities().is_empty());
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
        let mut hits = CacheHits::new();
        let result = resolve_workspace_id_inner(&mock, None, &None, &mut hits)
            .await
            .unwrap();
        assert_eq!(result, 99);
        assert!(hits.entities().is_empty());
    }
}
