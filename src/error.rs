use std::fmt;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    Api(String),
    HttpStatus { status: u16, body: String },
    Auth(String),
    Keyring(String),
    Cache(String),
    Io(String),
    NotFound(String),
    InvalidInput(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Api(msg) => write!(f, "API error: {msg}"),
            AppError::HttpStatus { status, body } => {
                write!(f, "HTTP {status}: {body}")
            }
            AppError::Auth(msg) => write!(f, "Authentication error: {msg}"),
            AppError::Keyring(msg) => write!(f, "Keyring error: {msg}"),
            AppError::Cache(msg) => write!(f, "Cache error: {msg}"),
            AppError::Io(msg) => write!(f, "IO error: {msg}"),
            AppError::NotFound(msg) => write!(f, "Not found: {msg}"),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Api(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Api(format!("JSON parse error: {e}"))
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError::Keyring(e.to_string())
    }
}

impl From<crate::cache::CacheError> for AppError {
    fn from(e: crate::cache::CacheError) -> Self {
        AppError::Cache(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_api_error() {
        let e = AppError::Api("msg".to_string());
        assert_eq!(e.to_string(), "API error: msg");
    }

    #[test]
    fn display_http_status() {
        let e = AppError::HttpStatus {
            status: 404,
            body: "not found".to_string(),
        };
        assert_eq!(e.to_string(), "HTTP 404: not found");
    }

    #[test]
    fn display_auth_error() {
        let e = AppError::Auth("msg".to_string());
        assert_eq!(e.to_string(), "Authentication error: msg");
    }

    #[test]
    fn display_keyring_error() {
        let e = AppError::Keyring("msg".to_string());
        assert_eq!(e.to_string(), "Keyring error: msg");
    }

    #[test]
    fn display_cache_error() {
        let e = AppError::Cache("msg".to_string());
        assert_eq!(e.to_string(), "Cache error: msg");
    }

    #[test]
    fn display_not_found() {
        let e = AppError::NotFound("msg".to_string());
        assert_eq!(e.to_string(), "Not found: msg");
    }

    #[test]
    fn display_invalid_input() {
        let e = AppError::InvalidInput("msg".to_string());
        assert_eq!(e.to_string(), "Invalid input: msg");
    }

    #[test]
    fn from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("not json").unwrap_err();
        let app_err = AppError::from(json_err);
        match &app_err {
            AppError::Api(msg) => assert!(msg.contains("JSON parse error:"), "got: {msg}"),
            other => panic!("expected Api, got: {other:?}"),
        }
    }

    #[test]
    fn from_keyring_error() {
        let kr_err = keyring::Error::NoEntry;
        let app_err = AppError::from(kr_err);
        match &app_err {
            AppError::Keyring(_) => {}
            other => panic!("expected Keyring, got: {other:?}"),
        }
    }

    #[test]
    fn from_cache_error() {
        use crate::cache::CacheError;
        let cache_err = CacheError::InvalidKey("bad".to_string());
        let app_err = AppError::from(cache_err);
        match &app_err {
            AppError::Cache(msg) => assert!(msg.contains("Invalid cache key"), "got: {msg}"),
            other => panic!("expected Cache, got: {other:?}"),
        }
    }

    #[test]
    fn error_trait_impl() {
        let e = AppError::Api("test".to_string());
        let _: &dyn std::error::Error = &e;
    }
}
