use chrono::TimeDelta;
use colored::Colorize;

use crate::cache::FileCache;
use crate::error::Result;

fn get_cache() -> Result<FileCache> {
    Ok(FileCache::default_with_ttl(TimeDelta::hours(72))?)
}

fn clear_inner(cache: &FileCache) -> Result<()> {
    cache.clear()?;
    println!("{} Cache cleared", "✓".green().bold());
    Ok(())
}

fn status_inner(cache: &FileCache) -> Result<()> {
    let statuses = cache.status();
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
    let cache = get_cache()?;
    clear_inner(&cache)
}

pub async fn status() -> Result<()> {
    let cache = get_cache()?;
    status_inner(&cache)
}

#[cfg(test)]
mod tests {
    use super::*;
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
