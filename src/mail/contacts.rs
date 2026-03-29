use crate::config::AppConfig;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn contacts_path() -> PathBuf {
    AppConfig::config_dir().join("contacts.json")
}

pub fn load_contacts() -> BTreeSet<String> {
    let path = contacts_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        BTreeSet::new()
    }
}

fn save_contacts(contacts: &BTreeSet<String>) {
    let path = contacts_path();
    if let Ok(data) = serde_json::to_string(contacts) {
        fs::write(path, data).ok();
    }
}

/// Add addresses to the contact book (called after successful send).
pub fn add_addresses(addresses: &[&str]) {
    let mut contacts = load_contacts();
    for addr in addresses {
        let trimmed = addr.trim();
        if !trimmed.is_empty() {
            contacts.insert(trimmed.to_string());
        }
    }
    save_contacts(&contacts);
}

/// Search contacts matching a query prefix.
pub fn search(query: &str) -> Vec<String> {
    if query.is_empty() {
        return Vec::new();
    }
    let lower = query.to_lowercase();
    load_contacts()
        .into_iter()
        .filter(|c| c.to_lowercase().contains(&lower))
        .take(10)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_search() {
        add_addresses(&["test_contact_search@example.com"]);
        let results = search("test_contact_search");
        assert!(!results.is_empty());
        assert!(results[0].contains("test_contact_search"));

        // Cleanup
        let mut contacts = load_contacts();
        contacts.remove("test_contact_search@example.com");
        save_contacts(&contacts);
    }
}
