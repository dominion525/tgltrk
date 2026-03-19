use colored::Colorize;

use crate::commands::CacheHits;
use crate::error::Result;
use serde::Serialize;
use std::fmt;

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

fn print_cache_hits_text(hits: &CacheHits) {
    for entity in hits.entities() {
        println!("{}", format!("(cached: {entity})").dimmed());
    }
}

pub fn print_result<T: Serialize + fmt::Display>(
    value: &T,
    json: bool,
    hits: &CacheHits,
) -> Result<()> {
    if json {
        print_json(&make_envelope(value, hits))
    } else {
        print_cache_hits_text(hits);
        println!("{value}");
        Ok(())
    }
}

pub fn print_list<T: Serialize + fmt::Display>(
    items: &[T],
    json: bool,
    hits: &CacheHits,
) -> Result<()> {
    if json {
        print_json(&make_envelope(items, hits))
    } else {
        print_cache_hits_text(hits);
        for item in items {
            println!("{item}");
        }
        Ok(())
    }
}

pub fn print_null(json: bool, hits: &CacheHits) -> Result<()> {
    if json {
        print_json(&make_envelope(Option::<()>::None, hits))
    } else {
        print_cache_hits_text(hits);
        Ok(())
    }
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Tag, User};

    fn make_user() -> User {
        User {
            email: "a@b.com".to_string(),
            fullname: "Alice".to_string(),
            default_workspace_id: 1,
            timezone: "UTC".to_string(),
        }
    }

    #[test]
    fn print_result_text_mode() {
        let user = make_user();
        let hits = CacheHits::new();
        assert!(print_result(&user, false, &hits).is_ok());
    }

    #[test]
    fn print_result_json_mode() {
        let user = make_user();
        let hits = CacheHits::new();
        assert!(print_result(&user, true, &hits).is_ok());
    }

    #[test]
    fn print_list_text_mode() {
        let tags = vec![
            Tag {
                id: 1,
                workspace_id: 1,
                name: "a".to_string(),
            },
            Tag {
                id: 2,
                workspace_id: 1,
                name: "b".to_string(),
            },
        ];
        let hits = CacheHits::new();
        assert!(print_list(&tags, false, &hits).is_ok());
    }

    #[test]
    fn print_json_serializes() {
        assert!(print_json(&vec![1, 2, 3]).is_ok());
    }

    #[test]
    fn print_null_json_mode() {
        let hits = CacheHits::new();
        assert!(print_null(true, &hits).is_ok());
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
}
