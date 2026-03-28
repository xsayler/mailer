use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountConfig {
    pub display_name: String,
    pub email: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    #[serde(default, skip_serializing)]
    pub password: String,
    #[serde(default)]
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub accounts: Vec<AccountConfig>,
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        let dir = dirs_fallback();
        fs::create_dir_all(&dir).ok();
        dir
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("accounts.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            fs::write(path, data).ok();
        }
    }

    pub fn first_account(&self) -> Option<&AccountConfig> {
        self.accounts.first()
    }
}

fn dirs_fallback() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("mailer")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("mailer")
    } else {
        PathBuf::from(".config/mailer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_ends_with_accounts_json() {
        let path = AppConfig::config_path();
        assert!(path.to_string_lossy().ends_with("accounts.json"));
    }

    #[test]
    fn config_dir_ends_with_mailer() {
        let dir = AppConfig::config_dir();
        assert!(dir.to_string_lossy().ends_with("mailer"));
    }

    #[test]
    fn default_config_has_no_accounts() {
        let config = AppConfig::default();
        assert!(config.accounts.is_empty());
        assert!(config.first_account().is_none());
    }

    #[test]
    fn first_account_returns_first() {
        let config = AppConfig {
            accounts: vec![
                AccountConfig {
                    display_name: "Alice".to_string(),
                    email: "alice@test.com".to_string(),
                    imap_host: "imap.test.com".to_string(),
                    imap_port: 993,
                    smtp_host: "smtp.test.com".to_string(),
                    smtp_port: 587,
                    username: "alice".to_string(),
                    password: String::new(),
                    signature: String::new(),
                },
                AccountConfig {
                    display_name: "Bob".to_string(),
                    email: "bob@test.com".to_string(),
                    imap_host: "imap.test.com".to_string(),
                    imap_port: 993,
                    smtp_host: "smtp.test.com".to_string(),
                    smtp_port: 587,
                    username: "bob".to_string(),
                    password: String::new(),
                    signature: String::new(),
                },
            ],
        };
        let first = config.first_account().unwrap();
        assert_eq!(first.email, "alice@test.com");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("mailer_test_config");
        fs::create_dir_all(&dir).ok();
        let path = dir.join("test_accounts.json");

        let config = AppConfig {
            accounts: vec![AccountConfig {
                display_name: "Test".to_string(),
                email: "test@example.com".to_string(),
                imap_host: "imap.example.com".to_string(),
                imap_port: 993,
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: 587,
                username: "testuser".to_string(),
                password: "secret123".to_string(),
                signature: String::new(),
            }],
        };

        // Manual save to test path
        let data = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&path, &data).unwrap();

        // Reload
        let loaded_data = fs::read_to_string(&path).unwrap();
        let loaded: AppConfig = serde_json::from_str(&loaded_data).unwrap();

        assert_eq!(loaded.accounts.len(), 1);
        assert_eq!(loaded.accounts[0].email, "test@example.com");
        assert_eq!(loaded.accounts[0].imap_port, 993);

        // Cleanup
        fs::remove_file(&path).ok();
        fs::remove_dir(&dir).ok();
    }

    #[test]
    fn password_skipped_in_serialization() {
        let config = AppConfig {
            accounts: vec![AccountConfig {
                display_name: "Test".to_string(),
                email: "test@example.com".to_string(),
                imap_host: "imap.test.com".to_string(),
                imap_port: 993,
                smtp_host: "smtp.test.com".to_string(),
                smtp_port: 587,
                username: "user".to_string(),
                password: "secret_password".to_string(),
                signature: String::new(),
            }],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(
            !json.contains("secret_password"),
            "password should not appear in serialized JSON"
        );
        assert!(!json.contains("password"), "password field should be skipped");
    }

    #[test]
    fn deserialization_without_password() {
        let json = r#"{"accounts":[{
            "display_name":"Test",
            "email":"test@example.com",
            "imap_host":"imap.test.com",
            "imap_port":993,
            "smtp_host":"smtp.test.com",
            "smtp_port":587,
            "username":"user"
        }]}"#;

        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.accounts[0].password, "");
    }

    #[test]
    fn dirs_fallback_returns_path() {
        let path = dirs_fallback();
        assert!(path.to_string_lossy().contains("mailer"));
    }
}

/// Store password in the system keyring.
pub async fn store_password(email: &str, password: &str) -> Result<(), String> {
    let keyring = oo7::Keyring::new()
        .await
        .map_err(|e| format!("Keyring error: {e}"))?;
    keyring
        .create_item(
            &format!("Mailer password for {email}"),
            &[("service", "com.sayler.mailer"), ("account", email)],
            password,
            true,
        )
        .await
        .map_err(|e| format!("Keyring store error: {e}"))?;
    Ok(())
}

/// Load password from the system keyring.
pub async fn load_password(email: &str) -> Result<String, String> {
    let keyring = oo7::Keyring::new()
        .await
        .map_err(|e| format!("Keyring error: {e}"))?;
    let items = keyring
        .search_items(&[("service", "com.sayler.mailer"), ("account", email)])
        .await
        .map_err(|e| format!("Keyring search error: {e}"))?;
    if let Some(item) = items.first() {
        let secret = item
            .secret()
            .await
            .map_err(|e| format!("Keyring read error: {e}"))?;
        Ok(String::from_utf8_lossy(secret.as_bytes()).to_string())
    } else {
        Err("Password not found in keyring".to_string())
    }
}

