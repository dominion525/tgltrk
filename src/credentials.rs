use crate::constants::{ENV_API_TOKEN, KEYRING_SERVICE, KEYRING_USER};
use crate::error::{AppError, Result};

#[cfg(test)]
use mockall::automock;

pub struct Credential {
    pub api_token: String,
}

#[cfg_attr(test, automock)]
pub trait CredentialStore {
    fn read(&self) -> Result<Credential>;
    fn save(&self, token: &str) -> Result<()>;
    fn clear(&self) -> Result<()>;
}

pub struct KeyringStore {
    entry: keyring::Entry,
}

impl KeyringStore {
    pub fn new() -> Result<Self> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        Ok(Self { entry })
    }
}

impl CredentialStore for KeyringStore {
    fn read(&self) -> Result<Credential> {
        let api_token = self
            .entry
            .get_password()
            .map_err(|e| AppError::Auth(format!("No saved token: {e}")))?;
        Ok(Credential { api_token })
    }

    fn save(&self, token: &str) -> Result<()> {
        self.entry
            .set_password(token)
            .map_err(|e| AppError::Keyring(format!("Failed to save token: {e}")))?;
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        self.entry
            .delete_credential()
            .map_err(|e| AppError::Keyring(format!("Failed to clear token: {e}")))?;
        Ok(())
    }
}

pub struct EnvStore {
    token: String,
}

impl CredentialStore for EnvStore {
    fn read(&self) -> Result<Credential> {
        Ok(Credential {
            api_token: self.token.clone(),
        })
    }

    fn save(&self, _token: &str) -> Result<()> {
        Err(AppError::Auth(
            "Cannot save token when using environment variable".to_string(),
        ))
    }

    fn clear(&self) -> Result<()> {
        Err(AppError::Auth(
            "Cannot clear token when using environment variable".to_string(),
        ))
    }
}

pub fn get_store() -> Result<Box<dyn CredentialStore>> {
    if let Ok(token) = std::env::var(ENV_API_TOKEN) {
        return Ok(Box::new(EnvStore { token }));
    }
    let store = KeyringStore::new().map_err(|e| {
        AppError::Keyring(format!(
            "Failed to initialize keyring: {e}. \
             Set the {ENV_API_TOKEN} environment variable as an alternative."
        ))
    })?;
    Ok(Box::new(store))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_store_read_returns_token() {
        let store = EnvStore {
            token: "abc".to_string(),
        };
        let cred = store.read().unwrap();
        assert_eq!(cred.api_token, "abc");
    }

    #[test]
    fn env_store_save_returns_error() {
        let store = EnvStore {
            token: "abc".to_string(),
        };
        let result = store.save("x");
        assert!(result.is_err());
    }

    #[test]
    fn env_store_clear_returns_error() {
        let store = EnvStore {
            token: "abc".to_string(),
        };
        let result = store.clear();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_store_with_env_var_returns_env_store() {
        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: env var access serialized by ENV_MUTEX
        unsafe { std::env::set_var("TOGGL_API_TOKEN", "my_test_token") };
        let store = get_store().unwrap();
        let cred = store.read().unwrap();
        assert_eq!(cred.api_token, "my_test_token");
        // SAFETY: env var access serialized by ENV_MUTEX
        unsafe { std::env::remove_var("TOGGL_API_TOKEN") };
    }
}
