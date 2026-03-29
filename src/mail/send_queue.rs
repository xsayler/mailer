use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub id: String,
    pub account_email: String,
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<(String, Vec<u8>)>,
}

fn queue_path() -> PathBuf {
    AppConfig::config_dir().join("send_queue.json")
}

pub fn enqueue(msg: QueuedMessage) {
    let mut queue = load_queue();
    queue.push(msg);
    save_queue(&queue);
}

pub fn load_queue() -> Vec<QueuedMessage> {
    let path = queue_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_queue(queue: &[QueuedMessage]) {
    let path = queue_path();
    if let Ok(data) = serde_json::to_string(queue) {
        fs::write(path, data).ok();
    }
}

pub fn remove_from_queue(id: &str) {
    let mut queue = load_queue();
    queue.retain(|m| m.id != id);
    save_queue(&queue);
}

pub fn is_empty() -> bool {
    load_queue().is_empty()
}
