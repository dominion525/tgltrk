use serde::Serialize;

use crate::cache::CacheHits;
use crate::cache::FileCache;
use crate::error::{AppError, Result};
use crate::output;

fn require_cache() -> Result<crate::cache::FileCache> {
    super::get_cache().ok_or_else(|| AppError::Cache("Failed to initialize cache".to_string()))
}

fn clear_inner(cache: &FileCache, json: bool) -> Result<()> {
    cache.clear()?;
    let hits = CacheHits::new();
    output::print_deleted(&mut std::io::stdout(), json, "Cache cleared", &hits)
}

#[derive(Serialize)]
struct CacheStatusEntry {
    key: String,
    size: u64,
    modified: Option<String>,
}

#[derive(Serialize)]
struct CacheStatus {
    cache_dir: String,
    entries: Vec<CacheStatusEntry>,
}

impl std::fmt::Display for CacheStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cache dir: {}", self.cache_dir)
    }
}

fn status_inner(cache: &FileCache, json: bool) -> Result<()> {
    let statuses = cache.status();
    if json {
        let data = CacheStatus {
            cache_dir: cache.cache_dir().display().to_string(),
            entries: statuses
                .iter()
                .map(|s| CacheStatusEntry {
                    key: s.key.clone(),
                    size: s.size,
                    modified: s.modified.map(|m| m.to_rfc3339()),
                })
                .collect(),
        };
        let hits = CacheHits::new();
        output::print_result(&mut std::io::stdout(), &data, json, &hits)
    } else if statuses.is_empty() {
        println!("Cache is empty");
        println!("Cache dir: {}", cache.cache_dir().display());
        Ok(())
    } else {
        println!("Cache dir: {}", cache.cache_dir().display());
        for s in &statuses {
            println!("  {s}");
        }
        Ok(())
    }
}

pub async fn clear(json: bool) -> Result<()> {
    let cache = require_cache()?;
    clear_inner(&cache, json)
}

pub async fn status(json: bool) -> Result<()> {
    let cache = require_cache()?;
    status_inner(&cache, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::FileCache;
    use chrono::TimeDelta;
    use tempfile::TempDir;

    fn make_cache(tmp: &TempDir) -> FileCache {
        FileCache::new(tmp.path().to_path_buf(), TimeDelta::hours(72))
    }

    #[test]
    fn clear_inner_empties_cache() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        cache.set("projects", &vec!["p1"]).unwrap();
        assert!(clear_inner(&cache, false).is_ok());
    }

    #[test]
    fn clear_inner_succeeds_on_empty() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        assert!(clear_inner(&cache, false).is_ok());
    }

    #[test]
    fn status_inner_empty_cache() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        assert!(status_inner(&cache, false).is_ok());
    }

    #[test]
    fn status_inner_with_entries() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        cache.set("projects", &vec!["p1"]).unwrap();
        cache.set("tags", &vec!["t1"]).unwrap();
        assert!(status_inner(&cache, false).is_ok());
    }
}
