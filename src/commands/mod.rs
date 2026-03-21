pub mod auth;
pub mod cache_cmd;
pub mod clients;
pub mod entries;
pub mod me;
pub mod projects;
pub mod tags;
pub mod timer;
pub mod workspaces;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::OnceLock;

use chrono::TimeDelta;

use crate::api::client::{ApiClient, TogglClient};
use crate::cache::FileCache;
use crate::constants::CACHE_TTL_HOURS;
use crate::error::Result;
use crate::models::WorkspaceId;

static TOKEN_FINGERPRINT: OnceLock<String> = OnceLock::new();

fn compute_fingerprint(token: &str) -> String {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn cache_key(key: &str) -> String {
    match TOKEN_FINGERPRINT.get() {
        Some(fp) => format!("{fp}_{key}"),
        None => key.to_string(),
    }
}

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
) -> Result<WorkspaceId> {
    if let Some(id) = workspace_override {
        return Ok(WorkspaceId(id));
    }
    let user = cached_fetch("user", hits, client.get_me()).await?;
    Ok(user.default_workspace_id)
}

pub fn build_client(base_url: Option<&str>) -> Result<TogglClient> {
    let store = crate::credentials::get_store()?;
    let cred = store.read()?;
    TOKEN_FINGERPRINT.get_or_init(|| compute_fingerprint(&cred.api_token));
    match base_url {
        Some(url) => TogglClient::new_with_base_url(&cred.api_token, url),
        None => TogglClient::new(&cred.api_token),
    }
}

pub fn invalidate_cache(key: &str) {
    if let Some(cache) = get_cache() {
        let full_key = cache_key(key);
        if let Err(e) = cache.invalidate(&full_key) {
            eprintln!("Warning: failed to invalidate cache key '{key}': {e}");
        }
    }
}

pub fn clear_all_cache() {
    if let Some(cache) = get_cache() {
        if let Err(e) = cache.clear() {
            eprintln!("Warning: failed to clear cache: {e}");
        }
    }
}

pub async fn cached_fetch<T, Fut>(key: &str, hits: &mut CacheHits, fetch: Fut) -> Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    Fut: std::future::Future<Output = Result<T>>,
{
    let full_key = cache_key(key);
    let cache = get_cache();
    if let Some(cached) = cache.as_ref().and_then(|c| c.get::<T>(&full_key)) {
        hits.record(key);
        return Ok(cached);
    }
    let value = fetch.await?;
    if let Some(c) = &cache {
        if let Err(e) = c.set(&full_key, &value) {
            eprintln!("Warning: failed to write cache key '{key}': {e}");
        }
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
        assert_eq!(result, WorkspaceId(42));
        assert!(hits.entities().is_empty());
    }

    // API fallback path is covered by wiremock integration tests
    // in each command module (entries, projects, tags, timer).
}
