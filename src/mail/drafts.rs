use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub id: String,
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
    pub timestamp: i64,
}

fn drafts_dir() -> PathBuf {
    AppConfig::config_dir().join("drafts")
}

pub fn save_draft(draft: &Draft) -> Result<(), String> {
    let dir = drafts_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create drafts dir: {e}"))?;
    let path = dir.join(format!("{}.json", draft.id));
    let data = serde_json::to_string_pretty(draft).map_err(|e| format!("Serialize failed: {e}"))?;
    fs::write(path, data).map_err(|e| format!("Write failed: {e}"))?;
    Ok(())
}

pub fn load_draft(id: &str) -> Result<Draft, String> {
    let path = drafts_dir().join(format!("{id}.json"));
    let data = fs::read_to_string(&path).map_err(|e| format!("Read failed: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Deserialize failed: {e}"))
}

pub fn list_drafts() -> Vec<Draft> {
    let dir = drafts_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut drafts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(draft) = serde_json::from_str::<Draft>(&data) {
                    drafts.push(draft);
                }
            }
        }
    }
    drafts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    drafts
}

pub fn delete_draft(id: &str) -> Result<(), String> {
    let path = drafts_dir().join(format!("{id}.json"));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_delete_roundtrip() {
        let draft = Draft {
            id: "test-draft-123".to_string(),
            to: "user@example.com".to_string(),
            cc: String::new(),
            bcc: String::new(),
            subject: "Test Draft".to_string(),
            body: "Draft body".to_string(),
            timestamp: 1000,
        };
        save_draft(&draft).unwrap();
        let loaded = load_draft("test-draft-123").unwrap();
        assert_eq!(loaded.subject, "Test Draft");
        assert_eq!(loaded.to, "user@example.com");
        delete_draft("test-draft-123").unwrap();
        assert!(load_draft("test-draft-123").is_err());
    }

    #[test]
    fn list_drafts_returns_sorted() {
        let d1 = Draft {
            id: "list-test-1".to_string(),
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
            subject: "Older".to_string(),
            body: String::new(),
            timestamp: 100,
        };
        let d2 = Draft {
            id: "list-test-2".to_string(),
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
            subject: "Newer".to_string(),
            body: String::new(),
            timestamp: 200,
        };
        save_draft(&d1).unwrap();
        save_draft(&d2).unwrap();

        let drafts = list_drafts();
        let test_drafts: Vec<_> = drafts.iter().filter(|d| d.id.starts_with("list-test-")).collect();
        assert!(test_drafts.len() >= 2);
        assert!(test_drafts[0].timestamp >= test_drafts[1].timestamp);

        delete_draft("list-test-1").unwrap();
        delete_draft("list-test-2").unwrap();
    }
}
