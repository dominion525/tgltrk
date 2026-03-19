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
