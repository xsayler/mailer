use crate::config::AppConfig;
use crate::mail::models::MailMessage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterField {
    From,
    To,
    Subject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    MoveTo(String),
    MarkRead,
    Star,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub name: String,
    pub field: FilterField,
    pub contains: String,
    pub action: FilterAction,
    pub enabled: bool,
}

fn filters_path() -> PathBuf {
    AppConfig::config_dir().join("filters.json")
}

pub fn load_filters() -> Vec<Filter> {
    let path = filters_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_filters(filters: &[Filter]) {
    let path = filters_path();
    if let Ok(data) = serde_json::to_string_pretty(filters) {
        fs::write(path, data).ok();
    }
}

/// Check which filter matches a message. Returns the first matching filter's action.
pub fn match_filter(msg: &MailMessage, filters: &[Filter]) -> Option<FilterAction> {
    for filter in filters {
        if !filter.enabled {
            continue;
        }
        let value = match filter.field {
            FilterField::From => msg.from_display().to_lowercase(),
            FilterField::To => msg.to.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ").to_lowercase(),
            FilterField::Subject => msg.subject.to_lowercase(),
        };
        if value.contains(&filter.contains.to_lowercase()) {
            return Some(filter.action.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_from_filter() {
        let filters = vec![Filter {
            name: "Test".to_string(),
            field: FilterField::From,
            contains: "spam@".to_string(),
            action: FilterAction::Delete,
            enabled: true,
        }];
        let msg = MailMessage {
            from: vec![crate::mail::models::Address {
                name: None,
                email: "spam@example.com".to_string(),
            }],
            ..Default::default()
        };
        assert!(matches!(match_filter(&msg, &filters), Some(FilterAction::Delete)));
    }

    #[test]
    fn disabled_filter_skipped() {
        let filters = vec![Filter {
            name: "Disabled".to_string(),
            field: FilterField::From,
            contains: "test@".to_string(),
            action: FilterAction::Star,
            enabled: false,
        }];
        let msg = MailMessage {
            from: vec![crate::mail::models::Address {
                name: None,
                email: "test@example.com".to_string(),
            }],
            ..Default::default()
        };
        assert!(match_filter(&msg, &filters).is_none());
    }

    #[test]
    fn subject_filter_match() {
        let filters = vec![Filter {
            name: "Promo".to_string(),
            field: FilterField::Subject,
            contains: "sale".to_string(),
            action: FilterAction::MoveTo("Promo".to_string()),
            enabled: true,
        }];
        let msg = MailMessage {
            subject: "Big Sale Today!".to_string(),
            ..Default::default()
        };
        assert!(matches!(match_filter(&msg, &filters), Some(FilterAction::MoveTo(_))));
    }
}
