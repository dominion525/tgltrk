use colored::Colorize;

use crate::cache::CacheHits;
use crate::error::Result;
use serde::Serialize;
use std::fmt;
use std::io::Write;

#[derive(Serialize)]
pub struct JsonEnvelope<T: Serialize> {
    pub meta: JsonMeta,
    pub data: T,
}

#[derive(Serialize)]
pub struct JsonMeta {
    pub cached: Vec<String>,
}

fn make_envelope<T: Serialize>(data: T, hits: &CacheHits) -> JsonEnvelope<T> {
    JsonEnvelope {
        meta: JsonMeta {
            cached: hits.entities().to_vec(),
        },
        data,
    }
}

pub fn write_cache_hits_text(w: &mut impl Write, hits: &CacheHits) -> std::io::Result<()> {
    for entity in hits.entities() {
        writeln!(w, "{}", format!("(cached: {entity})").dimmed())?;
    }
    Ok(())
}

pub fn print_result<T: Serialize + fmt::Display>(
    w: &mut impl Write,
    value: &T,
    json: bool,
    hits: &CacheHits,
) -> Result<()> {
    if json {
        write_json(w, &make_envelope(value, hits))
    } else {
        write_cache_hits_text(w, hits)?;
        writeln!(w, "{value}")?;
        Ok(())
    }
}

pub fn print_list<T: Serialize + fmt::Display>(
    w: &mut impl Write,
    items: &[T],
    json: bool,
    hits: &CacheHits,
) -> Result<()> {
    if json {
        write_json(w, &make_envelope(items, hits))
    } else {
        write_cache_hits_text(w, hits)?;
        for item in items {
            writeln!(w, "{item}")?;
        }
        Ok(())
    }
}

pub fn print_success<T: Serialize + fmt::Display>(
    w: &mut impl Write,
    value: &T,
    json: bool,
    message: &str,
    hits: &CacheHits,
) -> Result<()> {
    if json {
        write_json(w, &make_envelope(value, hits))
    } else {
        write_cache_hits_text(w, hits)?;
        writeln!(w, "{} {message}", "✓".green().bold())?;
        writeln!(w, "{value}")?;
        Ok(())
    }
}

pub fn print_deleted(
    w: &mut impl Write,
    json: bool,
    message: &str,
    hits: &CacheHits,
) -> Result<()> {
    if json {
        write_json(w, &make_envelope(Option::<()>::None, hits))
    } else {
        write_cache_hits_text(w, hits)?;
        writeln!(w, "{} {message}", "✓".green().bold())?;
        Ok(())
    }
}

pub fn print_null(w: &mut impl Write, json: bool, hits: &CacheHits) -> Result<()> {
    if json {
        write_json(w, &make_envelope(Option::<()>::None, hits))
    } else {
        write_cache_hits_text(w, hits)?;
        Ok(())
    }
}

fn write_json<T: Serialize + ?Sized>(w: &mut impl Write, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    writeln!(w, "{json}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Tag, TagId, User, WorkspaceId};

    fn make_user() -> User {
        User {
            email: "a@b.com".to_string(),
            fullname: "Alice".to_string(),
            default_workspace_id: WorkspaceId(1),
            timezone: "UTC".to_string(),
        }
    }

    #[test]
    fn print_result_text_mode() {
        let user = make_user();
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_result(&mut buf, &user, false, &hits).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Alice"), "got: {output}");
        assert!(output.contains("a@b.com"), "got: {output}");
    }

    #[test]
    fn print_result_json_mode() {
        let user = make_user();
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_result(&mut buf, &user, true, &hits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed["data"]["email"], "a@b.com");
        assert_eq!(parsed["data"]["fullname"], "Alice");
    }

    #[test]
    fn print_list_text_mode() {
        let tags = vec![
            Tag {
                id: TagId(1),
                workspace_id: WorkspaceId(1),
                name: "a".to_string(),
            },
            Tag {
                id: TagId(2),
                workspace_id: WorkspaceId(1),
                name: "b".to_string(),
            },
        ];
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_list(&mut buf, &tags, false, &hits).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("#1 a"), "got: {output}");
        assert!(output.contains("#2 b"), "got: {output}");
    }

    #[test]
    fn print_json_serializes() {
        let mut buf = Vec::new();
        write_json(&mut buf, &vec![1, 2, 3]).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn print_null_json_mode() {
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_null(&mut buf, true, &hits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed["data"].is_null());
    }

    #[test]
    fn json_envelope_includes_cache_hits() {
        let mut hits = CacheHits::new();
        hits.record("user");
        hits.record("projects");
        let envelope = make_envelope("test", &hits);
        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["meta"]["cached"],
            serde_json::json!(["user", "projects"])
        );
        assert_eq!(parsed["data"], "test");
    }

    #[test]
    fn json_envelope_empty_cache() {
        let hits = CacheHits::new();
        let envelope = make_envelope(vec![1, 2, 3], &hits);
        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["meta"]["cached"], serde_json::json!([]));
    }

    #[test]
    fn print_success_text_mode() {
        let user = make_user();
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_success(&mut buf, &user, false, "User fetched", &hits).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("User fetched"), "got: {output}");
        assert!(output.contains("Alice"), "got: {output}");
    }

    #[test]
    fn print_success_json_mode() {
        let user = make_user();
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_success(&mut buf, &user, true, "User fetched", &hits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(parsed["data"]["fullname"], "Alice");
    }

    #[test]
    fn print_deleted_text_mode() {
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_deleted(&mut buf, false, "Project #1 deleted", &hits).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Project #1 deleted"), "got: {output}");
    }

    #[test]
    fn print_deleted_json_mode() {
        let hits = CacheHits::new();
        let mut buf = Vec::new();
        print_deleted(&mut buf, true, "Project #1 deleted", &hits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed["data"].is_null());
    }
}
