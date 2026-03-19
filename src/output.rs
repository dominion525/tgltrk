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
