use colored::Colorize;

use crate::cache::FileCache;
use crate::error::{AppError, Result};

fn require_cache() -> Result<crate::cache::FileCache> {
    super::get_cache().ok_or_else(|| AppError::Cache("Failed to initialize cache".to_string()))
}

fn clear_inner(cache: &FileCache) -> Result<()> {
    cache.clear()?;
    println!("{} Cache cleared", "✓".green().bold());
    Ok(())
}

fn status_inner(cache: &FileCache) -> Result<()> {
    let statuses = cache.status(crate::constants::CACHE_KEYS);
    if statuses.is_empty() {
        println!("Cache is empty");
        println!("Cache dir: {}", cache.cache_dir().display());
    } else {
        println!("Cache dir: {}", cache.cache_dir().display());
        for s in &statuses {
            println!("  {s}");
        }
    }
    Ok(())
}

pub async fn clear() -> Result<()> {
    let cache = require_cache()?;
    clear_inner(&cache)
}

pub async fn status() -> Result<()> {
    let cache = require_cache()?;
    status_inner(&cache)
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
        assert!(clear_inner(&cache).is_ok());
    }

    #[test]
    fn clear_inner_succeeds_on_empty() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        assert!(clear_inner(&cache).is_ok());
    }

    #[test]
    fn status_inner_empty_cache() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        assert!(status_inner(&cache).is_ok());
    }

    #[test]
    fn status_inner_with_entries() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        cache.set("projects", &vec!["p1"]).unwrap();
        cache.set("tags", &vec!["t1"]).unwrap();
        assert!(status_inner(&cache).is_ok());
    }
}
