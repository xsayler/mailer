use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Ru,
}

static LOCALE: Lazy<Locale> = Lazy::new(|| {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANGUAGE"))
        .unwrap_or_default()
        .to_lowercase();

    if lang.starts_with("ru") {
        Locale::Ru
    } else {
        Locale::En
    }
});

pub fn locale() -> Locale {
    *LOCALE
}

pub fn t(key: &str) -> &'static str {
    let locale = *LOCALE;
    match (key, locale) {
        // Window
        ("app.title", Locale::En) => "Mailer",
        ("app.title", Locale::Ru) => "Почта",

        // Tooltips
        ("tooltip.new_message", Locale::En) => "New Message",
        ("tooltip.new_message", Locale::Ru) => "Новое сообщение",
        ("tooltip.refresh", Locale::En) => "Refresh",
        ("tooltip.refresh", Locale::Ru) => "Обновить",
        ("tooltip.reply", Locale::En) => "Reply",
        ("tooltip.reply", Locale::Ru) => "Ответить",
        ("tooltip.account_settings", Locale::En) => "Account Settings",
        ("tooltip.account_settings", Locale::Ru) => "Настройки аккаунта",
        ("tooltip.delete", Locale::En) => "Delete",
        ("tooltip.delete", Locale::Ru) => "Удалить",
        ("tooltip.mark_read", Locale::En) => "Mark as read",
        ("tooltip.mark_read", Locale::Ru) => "Отметить как прочитанное",
        ("tooltip.mark_unread", Locale::En) => "Mark as unread",
        ("tooltip.mark_unread", Locale::Ru) => "Отметить как непрочитанное",

        // Status bar
        ("status.not_connected", Locale::En) => "Not connected",
        ("status.not_connected", Locale::Ru) => "Нет подключения",
        ("status.connecting", Locale::En) => "Connecting...",
        ("status.connecting", Locale::Ru) => "Подключение...",
        ("status.connected", Locale::En) => "Connected",
        ("status.connected", Locale::Ru) => "Подключено",
        ("status.connection_failed", Locale::En) => "Connection failed",
        ("status.connection_failed", Locale::Ru) => "Ошибка подключения",
        ("status.refreshing", Locale::En) => "Refreshing...",
        ("status.refreshing", Locale::Ru) => "Обновление...",
        ("status.refresh_failed", Locale::En) => "Refresh failed",
        ("status.refresh_failed", Locale::Ru) => "Ошибка обновления",
        ("status.error_loading", Locale::En) => "Error loading messages",
        ("status.error_loading", Locale::Ru) => "Ошибка загрузки сообщений",
        ("status.deleting", Locale::En) => "Deleting...",
        ("status.deleting", Locale::Ru) => "Удаление...",
        ("status.moving", Locale::En) => "Moving...",
        ("status.moving", Locale::Ru) => "Перемещение...",
        ("status.idle", Locale::En) => "Watching for new mail...",
        ("status.idle", Locale::Ru) => "Ожидание новых писем...",
        ("status.loading_password", Locale::En) => "Loading credentials...",
        ("status.loading_password", Locale::Ru) => "Загрузка учётных данных...",

        // Toasts
        ("toast.no_account", Locale::En) => "No account connected",
        ("toast.no_account", Locale::Ru) => "Аккаунт не подключён",
        ("toast.deleted", Locale::En) => "Message deleted",
        ("toast.deleted", Locale::Ru) => "Сообщение удалено",
        ("toast.moved", Locale::En) => "Message moved",
        ("toast.moved", Locale::Ru) => "Сообщение перемещено",

        // Message view
        ("message.no_selected", Locale::En) => "No Message Selected",
        ("message.no_selected", Locale::Ru) => "Сообщение не выбрано",
        ("message.select_hint", Locale::En) => "Select a message to read it",
        ("message.select_hint", Locale::Ru) => "Выберите сообщение для чтения",
        ("message.no_content", Locale::En) => "(no content)",
        ("message.no_content", Locale::Ru) => "(нет содержимого)",
        ("message.unsubscribe", Locale::En) => "Unsubscribe",
        ("message.unsubscribe", Locale::Ru) => "Отписаться",

        // Compose window
        ("compose.title", Locale::En) => "New Message",
        ("compose.title", Locale::Ru) => "Новое сообщение",
        ("compose.reply_title", Locale::En) => "Reply",
        ("compose.reply_title", Locale::Ru) => "Ответ",
        ("compose.send", Locale::En) => "Send",
        ("compose.send", Locale::Ru) => "Отправить",
        ("compose.sending", Locale::En) => "Sending...",
        ("compose.sending", Locale::Ru) => "Отправка...",
        ("compose.to", Locale::En) => "To:",
        ("compose.to", Locale::Ru) => "Кому:",
        ("compose.cc", Locale::En) => "Cc:",
        ("compose.cc", Locale::Ru) => "Копия:",
        ("compose.subject", Locale::En) => "Subject:",
        ("compose.subject", Locale::Ru) => "Тема:",
        ("compose.quick_reply", Locale::En) => "Quick reply...",
        ("compose.quick_reply", Locale::Ru) => "Быстрый ответ...",
        ("compose.enter_recipient", Locale::En) => "Please enter a recipient",
        ("compose.enter_recipient", Locale::Ru) => "Введите получателя",
        ("compose.bold", Locale::En) => "Bold",
        ("compose.bold", Locale::Ru) => "Жирный",
        ("compose.italic", Locale::En) => "Italic",
        ("compose.italic", Locale::Ru) => "Курсив",
        ("compose.link", Locale::En) => "Insert Link",
        ("compose.link", Locale::Ru) => "Вставить ссылку",
        ("compose.link_url", Locale::En) => "URL:",
        ("compose.link_url", Locale::Ru) => "URL:",
        ("compose.link_text", Locale::En) => "Text:",
        ("compose.link_text", Locale::Ru) => "Текст:",

        // Account dialog
        ("account.setup_title", Locale::En) => "Account Setup",
        ("account.setup_title", Locale::Ru) => "Настройка аккаунта",
        ("account.save", Locale::En) => "Save",
        ("account.save", Locale::Ru) => "Сохранить",
        ("account.add_title", Locale::En) => "Add Email Account",
        ("account.add_title", Locale::Ru) => "Добавить почтовый аккаунт",
        ("account.personal", Locale::En) => "Personal",
        ("account.personal", Locale::Ru) => "Личные данные",
        ("account.display_name", Locale::En) => "Display Name",
        ("account.display_name", Locale::Ru) => "Отображаемое имя",
        ("account.email", Locale::En) => "Email Address",
        ("account.email", Locale::Ru) => "Адрес электронной почты",
        ("account.imap_group", Locale::En) => "Incoming Mail (IMAP)",
        ("account.imap_group", Locale::Ru) => "Входящая почта (IMAP)",
        ("account.imap_server", Locale::En) => "IMAP Server",
        ("account.imap_server", Locale::Ru) => "Сервер IMAP",
        ("account.imap_port", Locale::En) => "IMAP Port",
        ("account.imap_port", Locale::Ru) => "Порт IMAP",
        ("account.smtp_group", Locale::En) => "Outgoing Mail (SMTP)",
        ("account.smtp_group", Locale::Ru) => "Исходящая почта (SMTP)",
        ("account.smtp_server", Locale::En) => "SMTP Server",
        ("account.smtp_server", Locale::Ru) => "Сервер SMTP",
        ("account.smtp_port", Locale::En) => "SMTP Port",
        ("account.smtp_port", Locale::Ru) => "Порт SMTP",
        ("account.auth_group", Locale::En) => "Authentication",
        ("account.auth_group", Locale::Ru) => "Аутентификация",
        ("account.username", Locale::En) => "Username",
        ("account.username", Locale::Ru) => "Имя пользователя",
        ("account.password", Locale::En) => "Password",
        ("account.password", Locale::Ru) => "Пароль",
        ("account.fill_required", Locale::En) => "Please fill in all required fields",
        ("account.fill_required", Locale::Ru) => "Заполните все обязательные поля",
        ("account.switch", Locale::En) => "Switch Account",
        ("account.switch", Locale::Ru) => "Сменить аккаунт",
        ("account.no_accounts", Locale::En) => "No accounts configured",
        ("account.no_accounts", Locale::Ru) => "Нет настроенных аккаунтов",
        ("account.manage", Locale::En) => "Manage Accounts",
        ("account.manage", Locale::Ru) => "Управление аккаунтами",
        ("account.remove", Locale::En) => "Remove",
        ("account.remove", Locale::Ru) => "Удалить",
        ("account.confirm_remove", Locale::En) => "Remove this account?",
        ("account.confirm_remove", Locale::Ru) => "Удалить этот аккаунт?",

        // Sort
        ("sort.date", Locale::En) => "Date",
        ("sort.date", Locale::Ru) => "Дата",
        ("sort.sender", Locale::En) => "Sender",
        ("sort.sender", Locale::Ru) => "Отправитель",
        ("sort.subject", Locale::En) => "Subject",
        ("sort.subject", Locale::Ru) => "Тема",
        ("sort.unread", Locale::En) => "Unread",
        ("sort.unread", Locale::Ru) => "Непрочитанные",

        // Search
        ("search.placeholder", Locale::En) => "Search on server...",
        ("search.placeholder", Locale::Ru) => "Поиск на сервере...",
        ("search.no_results", Locale::En) => "No messages found",
        ("search.no_results", Locale::Ru) => "Сообщения не найдены",

        // Context menu
        ("menu.delete", Locale::En) => "Delete",
        ("menu.delete", Locale::Ru) => "Удалить",
        ("menu.move_to", Locale::En) => "Move to...",
        ("menu.move_to", Locale::Ru) => "Переместить в...",
        ("menu.mark_read", Locale::En) => "Mark as read",
        ("menu.mark_read", Locale::Ru) => "Отметить как прочитанное",
        ("menu.mark_unread", Locale::En) => "Mark as unread",
        ("menu.mark_unread", Locale::Ru) => "Отметить как непрочитанное",
        ("menu.reply", Locale::En) => "Reply",
        ("menu.reply", Locale::Ru) => "Ответить",
        ("menu.reply_all", Locale::En) => "Reply All",
        ("menu.reply_all", Locale::Ru) => "Ответить всем",
        ("menu.forward", Locale::En) => "Forward",
        ("menu.forward", Locale::Ru) => "Переслать",
        ("menu.star", Locale::En) => "Star",
        ("menu.star", Locale::Ru) => "Отметить звёздочкой",
        ("menu.unstar", Locale::En) => "Unstar",
        ("menu.unstar", Locale::Ru) => "Снять звёздочку",

        // Tooltips extras
        ("tooltip.reply_all", Locale::En) => "Reply All",
        ("tooltip.reply_all", Locale::Ru) => "Ответить всем",
        ("tooltip.forward", Locale::En) => "Forward",
        ("tooltip.forward", Locale::Ru) => "Переслать",

        // View source / export
        ("menu.view_source", Locale::En) => "View Source",
        ("menu.view_source", Locale::Ru) => "Исходный код письма",
        ("menu.save_eml", Locale::En) => "Save as .eml",
        ("menu.save_eml", Locale::Ru) => "Сохранить как .eml",

        // Bulk operations
        ("toast.deleted_count", Locale::En) => "Deleted",
        ("toast.deleted_count", Locale::Ru) => "Удалено",

        // Dialogs
        ("dialog.confirm_delete", Locale::En) => "Delete this message?",
        ("dialog.confirm_delete", Locale::Ru) => "Удалить это сообщение?",
        ("dialog.cancel", Locale::En) => "Cancel",
        ("dialog.cancel", Locale::Ru) => "Отмена",
        ("dialog.delete", Locale::En) => "Delete",
        ("dialog.delete", Locale::Ru) => "Удалить",

        // Offline
        ("status.offline", Locale::En) => "Offline (cached)",
        ("status.offline", Locale::Ru) => "Оффлайн (кэш)",

        // Compose extras
        ("compose.reply_all_title", Locale::En) => "Reply All",
        ("compose.reply_all_title", Locale::Ru) => "Ответ всем",
        ("compose.forward_title", Locale::En) => "Forward",
        ("compose.forward_title", Locale::Ru) => "Пересылка",

        // Connection
        ("status.reconnecting", Locale::En) => "Reconnecting...",
        ("status.reconnecting", Locale::Ru) => "Переподключение...",

        // Undo send
        ("toast.sending_undo", Locale::En) => "Sending message...",
        ("toast.sending_undo", Locale::Ru) => "Отправка сообщения...",
        ("toast.undo", Locale::En) => "Undo",
        ("toast.undo", Locale::Ru) => "Отменить",
        ("toast.send_cancelled", Locale::En) => "Send cancelled",
        ("toast.send_cancelled", Locale::Ru) => "Отправка отменена",

        // Attachments
        ("attachment.title", Locale::En) => "Attachments",
        ("attachment.title", Locale::Ru) => "Вложения",
        ("attachment.save", Locale::En) => "Save",
        ("attachment.save", Locale::Ru) => "Сохранить",
        ("attachment.attach", Locale::En) => "Attach File",
        ("attachment.attach", Locale::Ru) => "Прикрепить файл",
        ("attachment.remove", Locale::En) => "Remove",
        ("attachment.remove", Locale::Ru) => "Удалить",

        // Pagination
        ("pagination.load_more", Locale::En) => "Load more messages",
        ("pagination.load_more", Locale::Ru) => "Загрузить ещё",

        // Message view
        ("message.load_images", Locale::En) => "Load external images",
        ("message.load_images", Locale::Ru) => "Загрузить внешние изображения",

        // Compose extras
        ("compose.bcc", Locale::En) => "Bcc:",
        ("compose.bcc", Locale::Ru) => "Скрытая:",

        // Account extras
        ("account.signature_group", Locale::En) => "Signature",
        ("account.signature_group", Locale::Ru) => "Подпись",
        ("account.signature_hint", Locale::En) => "Added automatically to new messages",
        ("account.signature_hint", Locale::Ru) => "Автоматически добавляется к новым сообщениям",

        // Folder management
        ("folder.create", Locale::En) => "Create Folder",
        ("folder.create", Locale::Ru) => "Создать папку",
        ("folder.rename", Locale::En) => "Rename Folder",
        ("folder.rename", Locale::Ru) => "Переименовать папку",
        ("folder.delete", Locale::En) => "Delete Folder",
        ("folder.delete", Locale::Ru) => "Удалить папку",
        ("folder.name_prompt", Locale::En) => "Folder name",
        ("folder.name_prompt", Locale::Ru) => "Имя папки",
        ("folder.confirm_delete", Locale::En) => "Delete this folder?",
        ("folder.confirm_delete", Locale::Ru) => "Удалить эту папку?",

        // Draft
        ("draft.saved", Locale::En) => "Draft saved",
        ("draft.saved", Locale::Ru) => "Черновик сохранён",
        ("draft.open", Locale::En) => "Drafts",
        ("draft.open", Locale::Ru) => "Черновики",
        ("draft.empty", Locale::En) => "No drafts",
        ("draft.empty", Locale::Ru) => "Нет черновиков",
        ("draft.discard", Locale::En) => "Discard",
        ("draft.discard", Locale::Ru) => "Удалить",

        // Notifications
        ("notification.new_mail", Locale::En) => "New mail",
        ("notification.new_mail", Locale::Ru) => "Новое письмо",

        // Models
        ("model.unknown", Locale::En) => "(unknown)",
        ("model.unknown", Locale::Ru) => "(неизвестно)",
        ("model.no_subject", Locale::En) => "(no subject)",
        ("model.no_subject", Locale::Ru) => "(без темы)",

        _ => {
            log::warn!("missing i18n key: {key}");
            "???"
        }
    }
}

/// Format a translated string with arguments.
pub fn tf(key: &str, args: &[&str]) -> String {
    let template = match (key, *LOCALE) {
        ("status.loading_folder", Locale::En) => "Loading {}...",
        ("status.loading_folder", Locale::Ru) => "Загрузка {}...",

        ("status.message_count", Locale::En) => "{} of {} messages",
        ("status.message_count", Locale::Ru) => "{} из {} сообщений",

        ("error.generic", Locale::En) => "Error: {}",
        ("error.generic", Locale::Ru) => "Ошибка: {}",

        ("error.delete_failed", Locale::En) => "Delete failed: {}",
        ("error.delete_failed", Locale::Ru) => "Ошибка удаления: {}",

        ("error.move_failed", Locale::En) => "Move failed: {}",
        ("error.move_failed", Locale::Ru) => "Ошибка перемещения: {}",

        ("error.keyring_failed", Locale::En) => "Keyring error: {}",
        ("error.keyring_failed", Locale::Ru) => "Ошибка связки ключей: {}",

        ("compose.send_failed", Locale::En) => "Send failed: {}",
        ("compose.send_failed", Locale::Ru) => "Ошибка отправки: {}",

        ("message.from", Locale::En) => "From: {}",
        ("message.from", Locale::Ru) => "От: {}",
        ("message.to", Locale::En) => "To: {}",
        ("message.to", Locale::Ru) => "Кому: {}",
        ("message.cc", Locale::En) => "Cc: {}",
        ("message.cc", Locale::Ru) => "Копия: {}",

        ("compose.re_subject", Locale::En) => "Re: {}",
        ("compose.re_subject", Locale::Ru) => "Re: {}",

        ("compose.fwd_subject", Locale::En) => "Fwd: {}",
        ("compose.fwd_subject", Locale::Ru) => "Fwd: {}",

        ("app.title_unread", Locale::En) => "Mailer ({})",
        ("app.title_unread", Locale::Ru) => "Почта ({})",

        ("notification.new_mail_from", Locale::En) => "From: {}",
        ("notification.new_mail_from", Locale::Ru) => "От: {}",

        ("attachment.save_failed", Locale::En) => "Failed to save: {}",
        ("attachment.save_failed", Locale::Ru) => "Ошибка сохранения: {}",

        ("attachment.size_kb", Locale::En) => "{} KB",
        ("attachment.size_kb", Locale::Ru) => "{} КБ",

        ("attachment.size_mb", Locale::En) => "{} MB",
        ("attachment.size_mb", Locale::Ru) => "{} МБ",

        ("search.results_count", Locale::En) => "{} results",
        ("search.results_count", Locale::Ru) => "Результатов: {}",

        ("toast.folder_created", Locale::En) => "Folder created: {}",
        ("toast.folder_created", Locale::Ru) => "Папка создана: {}",
        ("toast.folder_renamed", Locale::En) => "Folder renamed: {}",
        ("toast.folder_renamed", Locale::Ru) => "Папка переименована: {}",
        ("toast.folder_deleted", Locale::En) => "Folder deleted: {}",
        ("toast.folder_deleted", Locale::Ru) => "Папка удалена: {}",

        _ => key,
    };

    let mut result = template.to_string();
    for arg in args {
        if let Some(pos) = result.find("{}") {
            result.replace_range(pos..pos + 2, arg);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_returns_string_for_known_key() {
        let result = t("app.title");
        assert!(result == "Mailer" || result == "Почта");
    }

    #[test]
    fn t_returns_fallback_for_unknown_key() {
        assert_eq!(t("nonexistent.key.12345"), "???");
    }

    #[test]
    fn tf_single_arg() {
        let result = tf("error.generic", &["test"]);
        // Should contain the arg regardless of locale
        assert!(result.contains("test"));
    }

    #[test]
    fn tf_multiple_args_not_consumed() {
        // error.generic has only one {}, extra args are ignored
        let result = tf("error.generic", &["first", "second"]);
        assert!(result.contains("first"));
        assert!(!result.contains("{}"));
    }

    #[test]
    fn tf_unknown_key_returns_key() {
        let result = tf("unknown.key.xyz", &["arg"]);
        // Unknown keys return the key itself as template
        assert_eq!(result, "unknown.key.xyz");
    }

    #[test]
    fn tf_message_count() {
        let result = tf("status.message_count", &["42", "100"]);
        assert!(result.contains("42"));
    }

    #[test]
    fn tf_no_args() {
        let result = tf("error.generic", &[]);
        // Should still have unreplaced {}
        assert!(result.contains("{}"));
    }

    #[test]
    fn locale_is_valid() {
        let l = locale();
        assert!(l == Locale::En || l == Locale::Ru);
    }

    #[test]
    fn t_all_tooltip_keys_exist() {
        for key in &[
            "tooltip.new_message",
            "tooltip.refresh",
            "tooltip.reply",
            "tooltip.account_settings",
            "tooltip.delete",
            "tooltip.mark_read",
            "tooltip.mark_unread",
        ] {
            assert_ne!(t(key), "???", "missing key: {key}");
        }
    }

    #[test]
    fn t_all_status_keys_exist() {
        for key in &[
            "status.not_connected",
            "status.connecting",
            "status.connected",
            "status.connection_failed",
            "status.refreshing",
            "status.refresh_failed",
            "status.error_loading",
            "status.deleting",
            "status.moving",
            "status.idle",
            "status.loading_password",
        ] {
            assert_ne!(t(key), "???", "missing key: {key}");
        }
    }

    #[test]
    fn t_all_compose_keys_exist() {
        for key in &[
            "compose.title",
            "compose.reply_title",
            "compose.send",
            "compose.sending",
            "compose.to",
            "compose.cc",
            "compose.subject",
            "compose.enter_recipient",
            "compose.bold",
            "compose.italic",
            "compose.link",
        ] {
            assert_ne!(t(key), "???", "missing key: {key}");
        }
    }

    #[test]
    fn tf_all_format_keys_produce_output() {
        for key in &[
            "status.loading_folder",
            "status.message_count",
            "error.generic",
            "error.delete_failed",
            "error.move_failed",
            "compose.send_failed",
            "message.from",
            "message.to",
            "compose.re_subject",
            "notification.new_mail_from",
            "attachment.save_failed",
            "attachment.size_kb",
            "attachment.size_mb",
        ] {
            let result = tf(key, &["test"]);
            assert!(result.contains("test"), "key {key} did not interpolate arg");
        }
    }
}
