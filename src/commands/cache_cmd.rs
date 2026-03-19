use chrono::TimeDelta;
use colored::Colorize;

use crate::cache::FileCache;
use crate::error::{AppError, Result};

fn get_cache() -> Result<FileCache> {
    FileCache::default_with_ttl(TimeDelta::hours(72)).map_err(|e| AppError::Cache(e.to_string()))
}

pub async fn clear() -> Result<()> {
    let cache = get_cache()?;
    cache.clear().map_err(|e| AppError::Cache(e.to_string()))?;
    println!("{} Cache cleared", "✓".green().bold());
    Ok(())
}

pub async fn status() -> Result<()> {
    let cache = get_cache()?;
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
