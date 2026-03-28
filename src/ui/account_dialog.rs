use crate::config::{self, AccountConfig, AppConfig};
use crate::i18n::t;
use crate::runtime;
use adw::prelude::*;

pub fn show_account_dialog(
    parent: &impl IsA<gtk::Window>,
    on_save: impl Fn(AccountConfig) + 'static,
) {
    let window = adw::Window::builder()
        .title(t("account.setup_title"))
        .default_width(500)
        .default_height(600)
        .transient_for(parent)
        .modal(true)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    let save_btn = gtk::Button::with_label(t("account.save"));
    save_btn.add_css_class("suggested-action");
    header.pack_end(&save_btn);
    main_box.append(&header);

    let toast_overlay = adw::ToastOverlay::new();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(16);
    content.set_margin_bottom(16);

    // Welcome
    let title = gtk::Label::new(Some(t("account.add_title")));
    title.add_css_class("title-1");
    content.append(&title);

    // Personal info
    let personal_group = adw::PreferencesGroup::new();
    personal_group.set_title(t("account.personal"));

    let name_row = adw::EntryRow::new();
    name_row.set_title(t("account.display_name"));
    personal_group.add(&name_row);

    let email_row = adw::EntryRow::new();
    email_row.set_title(t("account.email"));
    personal_group.add(&email_row);

    content.append(&personal_group);

    // IMAP settings
    let imap_group = adw::PreferencesGroup::new();
    imap_group.set_title(t("account.imap_group"));

    let imap_host_row = adw::EntryRow::new();
    imap_host_row.set_title(t("account.imap_server"));
    imap_group.add(&imap_host_row);

    let imap_port_row = adw::EntryRow::new();
    imap_port_row.set_title(t("account.imap_port"));
    imap_port_row.set_text("993");
    imap_group.add(&imap_port_row);

    content.append(&imap_group);

    // SMTP settings
    let smtp_group = adw::PreferencesGroup::new();
    smtp_group.set_title(t("account.smtp_group"));

    let smtp_host_row = adw::EntryRow::new();
    smtp_host_row.set_title(t("account.smtp_server"));
    smtp_group.add(&smtp_host_row);

    let smtp_port_row = adw::EntryRow::new();
    smtp_port_row.set_title(t("account.smtp_port"));
    smtp_port_row.set_text("587");
    smtp_group.add(&smtp_port_row);

    content.append(&smtp_group);

    // Auth
    let auth_group = adw::PreferencesGroup::new();
    auth_group.set_title(t("account.auth_group"));

    let username_row = adw::EntryRow::new();
    username_row.set_title(t("account.username"));
    auth_group.add(&username_row);

    let password_row = adw::PasswordEntryRow::new();
    password_row.set_title(t("account.password"));
    auth_group.add(&password_row);

    content.append(&auth_group);

    // Signature
    let sig_group = adw::PreferencesGroup::new();
    sig_group.set_title(t("account.signature_group"));
    sig_group.set_description(Some(t("account.signature_hint")));

    let sig_view = gtk::TextView::new();
    sig_view.set_wrap_mode(gtk::WrapMode::Word);
    sig_view.set_left_margin(8);
    sig_view.set_right_margin(8);
    sig_view.set_top_margin(8);
    sig_view.set_bottom_margin(8);
    let sig_frame = gtk::Frame::new(None);
    sig_frame.set_child(Some(&sig_view));
    sig_frame.set_height_request(100);

    let sig_row_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sig_row_box.set_margin_start(12);
    sig_row_box.set_margin_end(12);
    sig_row_box.set_margin_top(8);
    sig_row_box.set_margin_bottom(8);
    sig_row_box.append(&sig_frame);

    content.append(&sig_group);
    content.append(&sig_row_box);

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_child(Some(&content));
    scrolled.set_vexpand(true);

    toast_overlay.set_child(Some(&scrolled));
    main_box.append(&toast_overlay);
    window.set_content(Some(&main_box));

    let win = window.clone();
    let toast_ov = toast_overlay.clone();

    save_btn.connect_clicked(move |_| {
        let email = email_row.text().to_string();
        let imap_host = imap_host_row.text().to_string();
        let smtp_host = smtp_host_row.text().to_string();
        let username = username_row.text().to_string();
        let password = password_row.text().to_string();

        if email.is_empty() || imap_host.is_empty() || username.is_empty() || password.is_empty() {
            let toast = adw::Toast::new(t("account.fill_required"));
            toast_ov.add_toast(toast);
            return;
        }

        let sig_buffer = sig_view.buffer();
        let signature = sig_buffer
            .text(&sig_buffer.start_iter(), &sig_buffer.end_iter(), false)
            .to_string();

        let config = AccountConfig {
            display_name: name_row.text().to_string(),
            email: email.clone(),
            imap_host,
            imap_port: imap_port_row
                .text()
                .parse()
                .unwrap_or(993),
            smtp_host,
            smtp_port: smtp_port_row
                .text()
                .parse()
                .unwrap_or(587),
            username,
            password: password.clone(),
            signature,
        };

        // Save to config file (password excluded via skip_serializing)
        let mut app_config = AppConfig::load();
        app_config.accounts.push(config.clone());
        app_config.save();

        // Store password in keyring
        let email_for_keyring = email.clone();
        let password_for_keyring = password.clone();
        runtime::spawn_on_main(
            async move {
                config::store_password(&email_for_keyring, &password_for_keyring).await
            },
            |result| {
                if let Err(e) = result {
                    log::warn!("Failed to store password in keyring: {e}");
                }
            },
        );

        on_save(config);
        win.close();
    });

    window.present();
}
