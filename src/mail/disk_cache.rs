use crate::config::AppConfig;
use crate::mail::models::{MailFolder, MailMessage};
use std::fs;
use std::path::PathBuf;

fn cache_dir() -> PathBuf {
    AppConfig::config_dir().join("cache")
}

fn folder_cache_path(folder: &str) -> PathBuf {
    // Sanitize folder name for filesystem
    let safe_name: String = folder
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    cache_dir().join(format!("{safe_name}.json"))
}

fn folders_cache_path() -> PathBuf {
    cache_dir().join("_folders.json")
}

/// Save messages for a folder to disk.
pub fn save_messages(folder: &str, messages: &[MailMessage]) {
    let dir = cache_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = folder_cache_path(folder);
    if let Ok(data) = serde_json::to_string(messages) {
        fs::write(path, data).ok();
    }
}

/// Load cached messages for a folder from disk.
pub fn load_messages(folder: &str) -> Option<Vec<MailMessage>> {
    let path = folder_cache_path(folder);
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save folder list to disk.
pub fn save_folders(folders: &[MailFolder]) {
    let dir = cache_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = folders_cache_path();
    if let Ok(data) = serde_json::to_string(folders) {
        fs::write(path, data).ok();
    }
}

/// Load cached folder list from disk.
pub fn load_folders() -> Option<Vec<MailFolder>> {
    let path = folders_cache_path();
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_messages() {
        let msgs = vec![MailMessage {
            uid: 42,
            subject: "Test cached".to_string(),
            ..Default::default()
        }];
        save_messages("_test_cache_folder", &msgs);
        let loaded = load_messages("_test_cache_folder").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].uid, 42);
        assert_eq!(loaded[0].subject, "Test cached");

        // Cleanup
        let path = folder_cache_path("_test_cache_folder");
        fs::remove_file(path).ok();
    }

    #[test]
    fn load_nonexistent_returns_none() {
        assert!(load_messages("_nonexistent_test_folder_xyz").is_none());
    }

    #[test]
    fn save_and_load_folders() {
        let folders = vec![MailFolder {
            name: "INBOX".to_string(),
            path: "INBOX".to_string(),
            unread_count: 3,
        }];
        save_folders(&folders);
        let loaded = load_folders().unwrap();
        assert!(!loaded.is_empty());
        assert_eq!(loaded[0].name, "INBOX");

        // Don't remove — other tests may need it
    }
}
