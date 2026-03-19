use chrono::{DateTime, TimeDelta, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[allow(dead_code)]
pub enum CacheError {
    InvalidKey(String),
    Io {
        op: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::InvalidKey(key) => {
                write!(f, "Invalid cache key: '{key}' (only [a-zA-Z0-9_] allowed)")
            }
            CacheError::Io { op, path, source } => {
                write!(f, "Cache {op} failed for {}: {source}", path.display())
            }
            CacheError::Json { path, source } => {
                write!(f, "Cache JSON error for {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for CacheError {}

// ---------------------------------------------------------------------------
// CacheEntry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
struct CacheEntry<T> {
    data: T,
    cached_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Pure functions
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn is_expired(cached_at: DateTime<Utc>, ttl: TimeDelta, now: DateTime<Utc>) -> bool {
    now.signed_duration_since(cached_at) >= ttl
}

fn validate_key(key: &str) -> Result<(), CacheError> {
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(CacheError::InvalidKey(key.to_string()));
    }
    Ok(())
}

fn cache_file_path(cache_dir: &Path, key: &str) -> Result<PathBuf, CacheError> {
    validate_key(key)?;
    Ok(cache_dir.join(format!("{key}.json")))
}

// ---------------------------------------------------------------------------
// FileCache
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct FileCache {
    cache_dir: PathBuf,
    default_ttl: TimeDelta,
}

#[allow(dead_code)]
impl FileCache {
    pub fn new(cache_dir: PathBuf, default_ttl: TimeDelta) -> Self {
        Self {
            cache_dir,
            default_ttl,
        }
    }

    pub fn default_with_ttl(default_ttl: TimeDelta) -> Result<Self, CacheError> {
        let project_dirs =
            directories::ProjectDirs::from("com", "tgltrk", "tgltrk").ok_or(CacheError::Io {
                op: "locate",
                path: PathBuf::from("~"),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine cache directory",
                ),
            })?;
        Ok(Self::new(
            project_dirs.cache_dir().to_path_buf(),
            default_ttl,
        ))
    }

    pub fn try_get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        let path = cache_file_path(&self.cache_dir, key)?;

        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(CacheError::Io {
                    op: "read",
                    path,
                    source: e,
                });
            }
        };

        let entry: CacheEntry<T> = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(_) => {
                let _ = std::fs::remove_file(&path);
                return Ok(None);
            }
        };

        if is_expired(entry.cached_at, self.default_ttl, Utc::now()) {
            let _ = std::fs::remove_file(&path);
            return Ok(None);
        }

        Ok(Some(entry.data))
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.try_get(key).unwrap_or(None)
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), CacheError> {
        let path = cache_file_path(&self.cache_dir, key)?;

        std::fs::create_dir_all(&self.cache_dir).map_err(|e| CacheError::Io {
            op: "create_dir",
            path: self.cache_dir.clone(),
            source: e,
        })?;

        let entry = CacheEntry {
            data: value,
            cached_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&entry).map_err(|e| CacheError::Json {
            path: path.clone(),
            source: e,
        })?;

        let tmp_path = self.cache_dir.join(format!(".{key}.tmp.json"));
        std::fs::write(&tmp_path, json.as_bytes()).map_err(|e| CacheError::Io {
            op: "write_tmp",
            path: tmp_path.clone(),
            source: e,
        })?;

        std::fs::rename(&tmp_path, &path).map_err(|e| CacheError::Io {
            op: "rename",
            path: path.clone(),
            source: e,
        })?;

        Ok(())
    }

    pub fn invalidate(&self, key: &str) -> Result<(), CacheError> {
        let path = cache_file_path(&self.cache_dir, key)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CacheError::Io {
                op: "delete",
                path,
                source: e,
            }),
        }
    }

    pub fn clear(&self) -> Result<(), CacheError> {
        match std::fs::remove_dir_all(&self.cache_dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CacheError::Io {
                op: "clear",
                path: self.cache_dir.clone(),
                source: e,
            }),
        }
    }

    pub fn status(&self, keys: &[&str]) -> Vec<CacheFileStatus> {
        keys.iter()
            .filter_map(|key| {
                let path = cache_file_path(&self.cache_dir, key).ok()?;
                let meta = std::fs::metadata(&path).ok()?;
                Some(CacheFileStatus {
                    key: (*key).to_string(),
                    size: meta.len(),
                    modified: meta.modified().ok().map(|t| {
                        let dt: DateTime<Utc> = t.into();
                        dt
                    }),
                })
            })
            .collect()
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

pub struct CacheFileStatus {
    pub key: String,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

impl fmt::Display for CacheFileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let modified = self
            .modified
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        write!(
            f,
            "{}: {} bytes, last updated {}",
            self.key, self.size, modified
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_cache(tmp: &TempDir) -> FileCache {
        FileCache::new(tmp.path().to_path_buf(), TimeDelta::hours(72))
    }

    #[test]
    fn is_expired_returns_false_when_within_ttl() {
        let cached_at = Utc::now();
        let ttl = TimeDelta::hours(72);
        let now = cached_at + TimeDelta::hours(1);
        assert!(!is_expired(cached_at, ttl, now));
    }

    #[test]
    fn is_expired_returns_true_when_past_ttl() {
        let cached_at = Utc::now();
        let ttl = TimeDelta::hours(72);
        let now = cached_at + TimeDelta::hours(73);
        assert!(is_expired(cached_at, ttl, now));
    }

    #[test]
    fn is_expired_returns_true_at_exact_boundary() {
        let cached_at = Utc::now();
        let ttl = TimeDelta::hours(72);
        let now = cached_at + TimeDelta::hours(72);
        assert!(is_expired(cached_at, ttl, now));
    }

    #[test]
    fn validate_key_accepts_valid_keys() {
        assert!(validate_key("projects").is_ok());
        assert!(validate_key("tags").is_ok());
        assert!(validate_key("user_info").is_ok());
        assert!(validate_key("abc123").is_ok());
    }

    #[test]
    fn validate_key_rejects_empty_key() {
        assert!(validate_key("").is_err());
    }

    #[test]
    fn validate_key_rejects_special_characters() {
        assert!(validate_key("../etc/passwd").is_err());
        assert!(validate_key("my-key").is_err());
        assert!(validate_key("key with spaces").is_err());
        assert!(validate_key("key.json").is_err());
    }

    #[test]
    fn cache_file_path_generates_correct_path() {
        let dir = Path::new("/tmp/cache");
        let path = cache_file_path(dir, "projects").unwrap();
        assert_eq!(path, PathBuf::from("/tmp/cache/projects.json"));
    }

    #[test]
    fn cache_file_path_rejects_invalid_key() {
        let dir = Path::new("/tmp/cache");
        assert!(cache_file_path(dir, "../hack").is_err());
    }

    #[test]
    fn cache_entry_serde_roundtrip() {
        let entry = CacheEntry {
            data: vec!["hello".to_string(), "world".to_string()],
            cached_at: Utc::now(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let restored: CacheEntry<Vec<String>> = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.data, restored.data);
    }

    #[test]
    fn set_and_try_get_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        let data = vec!["rust", "is", "great"];
        cache.set("test_key", &data).unwrap();

        let result: Option<Vec<String>> = cache.try_get("test_key").unwrap();
        assert_eq!(
            result,
            Some(vec![
                "rust".to_string(),
                "is".to_string(),
                "great".to_string()
            ])
        );
    }

    #[test]
    fn try_get_returns_none_for_missing_key() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        let result: Option<String> = cache.try_get("nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn try_get_returns_none_for_expired_entry() {
        let tmp = TempDir::new().unwrap();
        let cache = FileCache::new(tmp.path().to_path_buf(), TimeDelta::zero());

        cache.set("expired_key", &"data").unwrap();

        let result: Option<String> = cache.try_get("expired_key").unwrap();
        assert_eq!(result, None);

        let path = tmp.path().join("expired_key.json");
        assert!(!path.exists());
    }

    #[test]
    fn try_get_recovers_from_corrupted_file() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        let path = tmp.path().join("corrupt.json");
        std::fs::write(&path, b"not valid json{{{").unwrap();

        let result: Option<String> = cache.try_get("corrupt").unwrap();
        assert_eq!(result, None);
        assert!(!path.exists());
    }

    #[test]
    fn get_returns_none_on_error_gracefully() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        let result: Option<String> = cache.get("../invalid");
        assert_eq!(result, None);
    }

    #[test]
    fn invalidate_removes_cache_file() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        cache.set("to_delete", &42).unwrap();
        assert!(tmp.path().join("to_delete.json").exists());

        cache.invalidate("to_delete").unwrap();
        assert!(!tmp.path().join("to_delete.json").exists());
    }

    #[test]
    fn invalidate_succeeds_for_nonexistent_key() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        cache.invalidate("nonexistent").unwrap();
    }

    #[test]
    fn clear_removes_entire_cache_directory() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        cache.set("a", &1).unwrap();
        cache.set("b", &2).unwrap();
        assert!(tmp.path().join("a.json").exists());

        cache.clear().unwrap();
        assert!(!tmp.path().exists());
    }

    #[test]
    fn clear_succeeds_when_directory_missing() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        std::fs::remove_dir_all(tmp.path()).unwrap();
        cache.clear().unwrap();
    }

    #[test]
    fn set_creates_cache_directory_if_missing() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested").join("cache");
        let cache = FileCache::new(nested.clone(), TimeDelta::hours(72));

        cache.set("key", &"value").unwrap();
        assert!(nested.join("key.json").exists());
    }

    #[test]
    fn status_returns_existing_entries() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);

        cache.set("projects", &vec!["p1"]).unwrap();
        cache.set("tags", &vec!["t1"]).unwrap();

        let statuses = cache.status(&["projects", "tags"]);
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn status_returns_empty_when_no_cache() {
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        let statuses = cache.status(&["user", "projects", "tags"]);
        assert!(statuses.is_empty());
    }

    #[test]
    fn cache_error_display_invalid_key() {
        let e = CacheError::InvalidKey("bad!".to_string());
        let msg = e.to_string();
        assert!(msg.contains("Invalid cache key"), "got: {msg}");
    }

    #[test]
    fn cache_error_display_io() {
        let e = CacheError::Io {
            op: "read",
            path: std::path::PathBuf::from("/tmp/test"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = e.to_string();
        assert!(msg.contains("Cache read failed"), "got: {msg}");
    }

    #[test]
    fn cache_error_display_json() {
        let json_err = serde_json::from_str::<String>("bad").unwrap_err();
        let e = CacheError::Json {
            path: std::path::PathBuf::from("/tmp/test.json"),
            source: json_err,
        };
        let msg = e.to_string();
        assert!(msg.contains("Cache JSON error"), "got: {msg}");
    }

    #[test]
    fn cache_file_status_display() {
        let status = CacheFileStatus {
            key: "projects".to_string(),
            size: 1024,
            modified: None,
        };
        let msg = format!("{status}");
        assert!(msg.contains("projects"), "got: {msg}");
        assert!(msg.contains("1024"), "got: {msg}");
    }

    #[cfg(unix)]
    #[test]
    fn try_get_io_error_returns_err() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        cache.set("locked", &"data").unwrap();

        let path = tmp.path().join("locked.json");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result: Result<Option<String>, CacheError> = cache.try_get("locked");
        // Restore permissions for cleanup
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn set_fails_on_readonly_dir() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("readonly");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::set_permissions(&cache_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

        let cache = FileCache::new(cache_dir.clone(), TimeDelta::hours(72));
        let result = cache.set("test", &"data");
        // Restore permissions for cleanup
        std::fs::set_permissions(&cache_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn invalidate_io_error() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let cache = make_cache(&tmp);
        cache.set("to_delete", &42).unwrap();

        // Make directory read-only to prevent file deletion
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = cache.invalidate("to_delete");
        // Restore permissions for cleanup
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
    }
}
