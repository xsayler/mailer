use crate::i18n::t;
use chrono::{DateTime, Utc};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MailFolder {
    pub name: String,
    pub path: String,
    pub unread_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Address {
    pub name: Option<String>,
    pub email: String,
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref name) = self.name {
            write!(f, "{} <{}>", name, self.email)
        } else {
            write!(f, "{}", self.email)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub content_id: Option<String>,
    pub size: usize,
    #[serde(skip)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MailMessage {
    pub uid: u32,
    pub subject: String,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub date: Option<DateTime<Utc>>,
    pub is_read: bool,
    pub is_flagged: bool,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<Attachment>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub list_unsubscribe: Option<String>,
    #[serde(skip)]
    pub raw_source: Option<String>,
}

impl MailMessage {
    pub fn from_display(&self) -> String {
        self.from
            .first()
            .map(|a| a.name.clone().unwrap_or_else(|| a.email.clone()))
            .unwrap_or_else(|| t("model.unknown").to_string())
    }

    pub fn date_display(&self) -> String {
        self.date
            .map(|d| {
                let local = d.with_timezone(&chrono::Local);
                let now = chrono::Local::now();
                if local.date_naive() == now.date_naive() {
                    local.format("%H:%M").to_string()
                } else if local.date_naive() == (now - chrono::Duration::days(1)).date_naive() {
                    local.format("%H:%M").to_string() + " ▪ " + if crate::i18n::locale() == crate::i18n::Locale::Ru { "вчера" } else { "yesterday" }
                } else {
                    local.format("%d %b %Y").to_string()
                }
            })
            .unwrap_or_default()
    }

    /// Normalized subject for thread grouping (strips Re:/Fwd: prefixes).
    pub fn thread_subject(&self) -> String {
        let mut s = self.subject.trim().to_lowercase();
        loop {
            let trimmed = s.trim_start();
            if trimmed.starts_with("re:") || trimmed.starts_with("re :") {
                s = trimmed[3..].trim_start().to_string();
            } else if trimmed.starts_with("fwd:") || trimmed.starts_with("fwd :") {
                s = trimmed[4..].trim_start().to_string();
            } else {
                break;
            }
        }
        s
    }

    pub fn preview_text(&self) -> String {
        if let Some(ref text) = self.body_text {
            text.chars().take(200).collect()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn address_display_with_name() {
        let addr = Address {
            name: Some("John Doe".to_string()),
            email: "john@example.com".to_string(),
        };
        assert_eq!(addr.to_string(), "John Doe <john@example.com>");
    }

    #[test]
    fn address_display_without_name() {
        let addr = Address {
            name: None,
            email: "john@example.com".to_string(),
        };
        assert_eq!(addr.to_string(), "john@example.com");
    }

    #[test]
    fn address_display_empty_email() {
        let addr = Address {
            name: None,
            email: String::new(),
        };
        assert_eq!(addr.to_string(), "");
    }

    #[test]
    fn message_from_display_with_name() {
        let msg = MailMessage {
            from: vec![Address {
                name: Some("Alice".to_string()),
                email: "alice@test.com".to_string(),
            }],
            ..Default::default()
        };
        assert_eq!(msg.from_display(), "Alice");
    }

    #[test]
    fn message_from_display_without_name() {
        let msg = MailMessage {
            from: vec![Address {
                name: None,
                email: "alice@test.com".to_string(),
            }],
            ..Default::default()
        };
        assert_eq!(msg.from_display(), "alice@test.com");
    }

    #[test]
    fn message_from_display_empty() {
        let msg = MailMessage::default();
        // Should return the translated "(unknown)" string
        assert!(!msg.from_display().is_empty());
    }

    #[test]
    fn message_date_display_some() {
        let msg = MailMessage {
            date: Some(Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap()),
            ..Default::default()
        };
        let display = msg.date_display();
        assert!(display.contains("2024"));
    }

    #[test]
    fn message_date_display_none() {
        let msg = MailMessage::default();
        assert_eq!(msg.date_display(), "");
    }

    #[test]
    fn preview_text_short() {
        let msg = MailMessage {
            body_text: Some("Hello world".to_string()),
            ..Default::default()
        };
        assert_eq!(msg.preview_text(), "Hello world");
    }

    #[test]
    fn preview_text_long() {
        let long_text: String = "a".repeat(500);
        let msg = MailMessage {
            body_text: Some(long_text),
            ..Default::default()
        };
        assert_eq!(msg.preview_text().len(), 200);
    }

    #[test]
    fn preview_text_none() {
        let msg = MailMessage::default();
        assert_eq!(msg.preview_text(), "");
    }

    #[test]
    fn mail_folder_default() {
        let folder = MailFolder::default();
        assert_eq!(folder.unread_count, 0);
        assert!(folder.name.is_empty());
        assert!(folder.path.is_empty());
    }

    #[test]
    fn attachment_default() {
        let att = Attachment::default();
        assert!(att.filename.is_empty());
        assert!(att.data.is_empty());
        assert_eq!(att.size, 0);
    }
}

// GObject wrapper for use in gio::ListStore
mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::MailMessageObject)]
    pub struct MailMessageObject {
        #[property(get, set)]
        pub uid: RefCell<u32>,
        #[property(get, set)]
        pub subject: RefCell<String>,
        #[property(get, set)]
        pub from_display: RefCell<String>,
        #[property(get, set)]
        pub date_display: RefCell<String>,
        #[property(get, set)]
        pub is_read: RefCell<bool>,
        #[property(get, set)]
        pub is_flagged: RefCell<bool>,
        #[property(get, set)]
        pub has_attachments: RefCell<bool>,
        #[property(get, set)]
        pub thread_count: RefCell<u32>,
        #[property(get, set)]
        pub preview: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MailMessageObject {
        const NAME: &'static str = "MailMessageObject";
        type Type = super::MailMessageObject;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MailMessageObject {}
}

glib::wrapper! {
    pub struct MailMessageObject(ObjectSubclass<imp::MailMessageObject>);
}

impl MailMessageObject {
    pub fn new(msg: &MailMessage) -> Self {
        glib::Object::builder()
            .property("uid", msg.uid)
            .property("subject", &msg.subject)
            .property("from-display", msg.from_display())
            .property("date-display", msg.date_display())
            .property("is-read", msg.is_read)
            .property("is-flagged", msg.is_flagged)
            .property("has-attachments", !msg.attachments.is_empty())
            .property("thread-count", 0u32)
            .property("preview", msg.preview_text())
            .build()
    }
}
