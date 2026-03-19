pub mod auth;
pub mod cache_cmd;
pub mod entries;
pub mod me;
pub mod projects;
pub mod tags;
pub mod timer;

use chrono::TimeDelta;

use crate::api::client::{ApiClient, TogglClient};
use crate::cache::FileCache;
use crate::constants::CACHE_TTL_HOURS;
use crate::error::Result;

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

pub fn get_cache() -> Option<FileCache> {
    FileCache::default_with_ttl(TimeDelta::hours(CACHE_TTL_HOURS)).ok()
}

pub async fn resolve_workspace_id(
    client: &(impl ApiClient + ?Sized),
    workspace_override: Option<i64>,
    hits: &mut CacheHits,
) -> Result<i64> {
    if let Some(id) = workspace_override {
        return Ok(id);
    }
    let user = cached_fetch("user", hits, client.get_me()).await?;
    Ok(user.default_workspace_id)
}

pub fn build_client(base_url: Option<&str>) -> Result<TogglClient> {
    let store = crate::credentials::get_store()?;
    let cred = store.read()?;
    match base_url {
        Some(url) => TogglClient::new_with_base_url(&cred.api_token, url),
        None => TogglClient::new(&cred.api_token),
    }
}

pub fn invalidate_cache(key: &str) {
    if let Some(cache) = get_cache() {
        let _ = cache.invalidate(key);
    }
}

pub async fn cached_fetch<T, Fut>(key: &str, hits: &mut CacheHits, fetch: Fut) -> Result<T>
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
        let result = resolve_workspace_id(&mock, Some(42), &mut hits)
            .await
            .unwrap();
        assert_eq!(result, 42);
        assert!(hits.entities().is_empty());
    }

    // API fallback path is covered by wiremock integration tests
    // in each command module (entries, projects, tags, timer).
}
