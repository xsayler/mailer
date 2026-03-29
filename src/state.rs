use crate::mail::models::MailMessage;
use lru::LruCache;
use std::num::NonZeroUsize;

const MAX_CACHE_SIZE: usize = 500;

#[derive(Debug)]
pub struct AppState {
    pub selected_folder: Option<String>,
    pub messages: Vec<MailMessage>,
    pub folder_total: u32,
    pub loaded_count: u32,
    message_cache: LruCache<(String, u32), MailMessage>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_folder: None,
            messages: Vec::new(),
            folder_total: 0,
            loaded_count: 0,
            message_cache: LruCache::new(
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap_or(NonZeroUsize::MIN)
            ),
        }
    }

    pub fn set_messages(&mut self, folder: &str, messages: Vec<MailMessage>, append: bool) {
        for msg in &messages {
            self.message_cache
                .put((folder.to_string(), msg.uid), msg.clone());
        }
        if append {
            self.messages.extend(messages);
        } else {
            self.messages = messages;
        }
        self.loaded_count = self.messages.len() as u32;
    }

    pub fn has_more(&self) -> bool {
        self.loaded_count < self.folder_total
    }

    pub fn remove_message(&mut self, uid: u32) {
        self.messages.retain(|m| m.uid != uid);
        if let Some(ref folder) = self.selected_folder {
            self.message_cache.pop(&(folder.clone(), uid));
        }
        self.loaded_count = self.messages.len() as u32;
    }

    pub fn update_flagged_status(&mut self, uid: u32, flagged: bool) {
        for msg in &mut self.messages {
            if msg.uid == uid {
                msg.is_flagged = flagged;
            }
        }
        if let Some(ref folder) = self.selected_folder {
            if let Some(msg) = self.message_cache.get_mut(&(folder.clone(), uid)) {
                msg.is_flagged = flagged;
            }
        }
    }

    pub fn update_read_status(&mut self, uid: u32, read: bool) {
        for msg in &mut self.messages {
            if msg.uid == uid {
                msg.is_read = read;
            }
        }
        if let Some(ref folder) = self.selected_folder {
            if let Some(msg) = self.message_cache.get_mut(&(folder.clone(), uid)) {
                msg.is_read = read;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(uid: u32, subject: &str) -> MailMessage {
        MailMessage {
            uid,
            subject: subject.to_string(),
            is_read: false,
            ..Default::default()
        }
    }

    #[test]
    fn new_state_is_empty() {
        let state = AppState::new();
        assert!(state.messages.is_empty());
        assert!(state.selected_folder.is_none());
        assert_eq!(state.message_cache.len(), 0);
    }

    #[test]
    fn set_messages_populates_cache() {
        let mut state = AppState::new();
        let msgs = vec![make_msg(1, "Hello"), make_msg(2, "World")];
        state.set_messages("INBOX", msgs, false);

        assert_eq!(state.messages.len(), 2);
        assert!(state.message_cache.contains(&("INBOX".to_string(), 1)));
        assert!(state.message_cache.contains(&("INBOX".to_string(), 2)));
        assert!(!state.message_cache.contains(&("INBOX".to_string(), 99)));
    }

    #[test]
    fn set_messages_append_mode() {
        let mut state = AppState::new();
        state.set_messages("INBOX", vec![make_msg(1, "First")], false);
        state.set_messages("INBOX", vec![make_msg(2, "Second")], true);

        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.loaded_count, 2);
    }

    #[test]
    fn remove_message_from_list() {
        let mut state = AppState::new();
        state.selected_folder = Some("INBOX".to_string());
        state.set_messages("INBOX", vec![make_msg(1, "A"), make_msg(2, "B"), make_msg(3, "C")], false);

        state.remove_message(2);

        assert_eq!(state.messages.len(), 2);
        assert!(state.messages.iter().all(|m| m.uid != 2));
        assert!(!state.message_cache.contains(&("INBOX".to_string(), 2)));
        assert!(state.message_cache.contains(&("INBOX".to_string(), 1)));
    }

    #[test]
    fn remove_message_nonexistent() {
        let mut state = AppState::new();
        state.set_messages("INBOX", vec![make_msg(1, "A")], false);
        state.remove_message(99);
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn update_read_status_in_list_and_cache() {
        let mut state = AppState::new();
        state.selected_folder = Some("INBOX".to_string());
        state.set_messages("INBOX", vec![make_msg(1, "Test")], false);

        assert!(!state.messages[0].is_read);
        assert!(!state.message_cache.get(&("INBOX".to_string(), 1)).unwrap().is_read);

        state.update_read_status(1, true);

        assert!(state.messages[0].is_read);
        assert!(state.message_cache.get(&("INBOX".to_string(), 1)).unwrap().is_read);
    }

    #[test]
    fn update_read_status_nonexistent() {
        let mut state = AppState::new();
        state.set_messages("INBOX", vec![make_msg(1, "Test")], false);
        state.update_read_status(99, true);
        assert!(!state.messages[0].is_read);
    }

    #[test]
    fn cache_persists_across_set_messages() {
        let mut state = AppState::new();
        state.set_messages("INBOX", vec![make_msg(1, "First")], false);
        state.set_messages("Sent", vec![make_msg(2, "Second")], false);

        assert!(state.message_cache.contains(&("INBOX".to_string(), 1)));
        assert!(state.message_cache.contains(&("Sent".to_string(), 2)));
        // messages list only has the latest set
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].uid, 2);
    }

    #[test]
    fn has_more_returns_correct() {
        let mut state = AppState::new();
        state.folder_total = 100;
        state.set_messages("INBOX", vec![make_msg(1, "A")], false);
        assert!(state.has_more());

        state.folder_total = 1;
        assert!(!state.has_more());
    }

    #[test]
    fn lru_eviction_on_overflow() {
        let mut state = AppState::new();
        for i in 0..MAX_CACHE_SIZE as u32 + 1 {
            state.set_messages("INBOX", vec![make_msg(i, &format!("msg {i}"))], false);
        }
        // First entry should be evicted
        assert!(!state.message_cache.contains(&("INBOX".to_string(), 0)));
        // Last entry should exist
        assert!(state.message_cache.contains(&("INBOX".to_string(), MAX_CACHE_SIZE as u32)));
    }
}
