use crate::error::Result;
use serde::Serialize;
use std::fmt;

pub fn print_result<T: Serialize + fmt::Display>(value: &T, json: bool) -> Result<()> {
    if json {
        print_json(value)
    } else {
        println!("{value}");
        Ok(())
    }
}

pub fn print_list<T: Serialize + fmt::Display>(items: &[T], json: bool) -> Result<()> {
    if json {
        print_json(items)
    } else {
        for item in items {
            println!("{item}");
        }
        Ok(())
    }
}

pub fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
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
        assert!(print_result(&user, false).is_ok());
    }

    #[test]
    fn print_result_json_mode() {
        let user = make_user();
        assert!(print_result(&user, true).is_ok());
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
        assert!(print_list(&tags, false).is_ok());
    }

    #[test]
    fn print_json_serializes() {
        assert!(print_json(&vec![1, 2, 3]).is_ok());
    }
}
