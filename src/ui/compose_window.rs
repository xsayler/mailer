use crate::config::AccountConfig;
use crate::i18n::{t, tf};
use crate::mail::imap_client::ImapClient;
use crate::mail::smtp_client::SmtpClient;
use crate::runtime;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub fn show_compose_window(
    parent: &impl IsA<gtk::Window>,
    config: AccountConfig,
    draft: Option<crate::mail::drafts::Draft>,
    imap_client: Option<Arc<ImapClient>>,
    parent_toast: adw::ToastOverlay,
) {
    let window = adw::Window::builder()
        .title(t("compose.title"))
        .default_width(640)
        .default_height(500)
        .transient_for(parent)
        .modal(true)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Header bar
    let header = adw::HeaderBar::new();
    let send_btn = gtk::Button::with_label(t("compose.send"));
    send_btn.add_css_class("suggested-action");
    header.pack_end(&send_btn);

    let attach_btn = gtk::Button::from_icon_name("mail-attachment-symbolic");
    attach_btn.set_tooltip_text(Some(t("attachment.attach")));
    header.pack_start(&attach_btn);

    main_box.append(&header);

    // Form
    let form_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    form_box.set_margin_start(12);
    form_box.set_margin_end(12);
    form_box.set_margin_top(8);

    let to_entry = create_field_row(t("compose.to"));
    form_box.append(&to_entry.0);

    let cc_entry = create_field_row(t("compose.cc"));
    form_box.append(&cc_entry.0);

    let bcc_entry = create_field_row(t("compose.bcc"));
    form_box.append(&bcc_entry.0);

    let subject_entry = create_field_row(t("compose.subject"));
    form_box.append(&subject_entry.0);

    // Attachment chips area
    let attach_area = gtk::FlowBox::new();
    attach_area.set_selection_mode(gtk::SelectionMode::None);
    attach_area.set_margin_top(4);
    attach_area.set_margin_bottom(4);
    attach_area.set_visible(false);
    form_box.append(&attach_area);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.set_margin_top(4);
    form_box.append(&separator);

    // Formatting toolbar
    let fmt_bar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    fmt_bar.set_margin_top(4);
    fmt_bar.set_margin_bottom(4);

    let bold_btn = gtk::Button::with_label(t("compose.bold"));
    bold_btn.add_css_class("flat");
    bold_btn.add_css_class("caption");
    fmt_bar.append(&bold_btn);

    let italic_btn = gtk::Button::with_label(t("compose.italic"));
    italic_btn.add_css_class("flat");
    italic_btn.add_css_class("caption");
    fmt_bar.append(&italic_btn);

    let link_btn = gtk::Button::with_label(t("compose.link"));
    link_btn.add_css_class("flat");
    link_btn.add_css_class("caption");
    fmt_bar.append(&link_btn);

    form_box.append(&fmt_bar);

    // Body with formatting tags
    let body_view = gtk::TextView::new();
    body_view.set_wrap_mode(gtk::WrapMode::Word);
    body_view.set_left_margin(8);
    body_view.set_right_margin(8);
    body_view.set_top_margin(8);
    body_view.set_bottom_margin(8);

    let buffer = body_view.buffer();
    buffer.create_tag(Some("bold"), &[("weight", &700i32)]);
    buffer.create_tag(Some("italic"), &[("style", &gtk::pango::Style::Italic)]);
    buffer.create_tag(Some("link"), &[
        ("underline", &gtk::pango::Underline::Single),
        ("foreground", &"#5294e2"),
    ]);

    let body_scroll = gtk::ScrolledWindow::new();
    body_scroll.set_child(Some(&body_view));
    body_scroll.set_vexpand(true);

    form_box.append(&body_scroll);
    main_box.append(&form_box);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&main_box));
    window.set_content(Some(&toast_overlay));

    // Attachments storage
    let attachments: Rc<RefCell<Vec<(String, Vec<u8>)>>> = Rc::new(RefCell::new(Vec::new()));
    let sent_flag = Rc::new(std::cell::Cell::new(false));

    // Drag & drop files into compose
    {
        let drop_target = gtk::DropTarget::new(gtk::gio::File::static_type(), gtk::gdk::DragAction::COPY);
        let atts_drop = attachments.clone();
        let area_drop = attach_area.clone();
        drop_target.connect_drop(move |_, value, _x, _y| {
            if let Ok(file) = value.get::<gtk::gio::File>() {
                if let Some(path) = file.path() {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if let Ok(data) = std::fs::read(&path) {
                        atts_drop.borrow_mut().push((filename.clone(), data));
                        let chip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                        let lbl = gtk::Label::new(Some(&filename));
                        lbl.add_css_class("caption");
                        chip.append(&lbl);
                        let remove_btn = gtk::Button::from_icon_name("window-close-symbolic");
                        remove_btn.add_css_class("flat");
                        remove_btn.add_css_class("circular");
                        let atts2 = atts_drop.clone();
                        let fname = filename.clone();
                        remove_btn.connect_clicked(move |_| {
                            atts2.borrow_mut().retain(|(n, _)| n != &fname);
                        });
                        chip.append(&remove_btn);
                        area_drop.append(&chip);
                        area_drop.set_visible(true);
                        return true;
                    }
                }
            }
            false
        });
        window.add_controller(drop_target);
    }

    // Save draft on close if not sent
    {
        let to_d = to_entry.1.clone();
        let cc_d = cc_entry.1.clone();
        let bcc_d = bcc_entry.1.clone();
        let subj_d = subject_entry.1.clone();
        let body_d = body_view.clone();
        let sent = sent_flag.clone();
        let draft_id = draft.as_ref().map(|d| d.id.clone());
        window.connect_close_request(move |_| {
            if sent.get() {
                // Delete draft if we were editing one
                if let Some(ref id) = draft_id {
                    crate::mail::drafts::delete_draft(id).ok();
                }
                return glib::Propagation::Proceed;
            }
            let to = to_d.text().to_string();
            let subject = subj_d.text().to_string();
            let buffer = body_d.buffer();
            let body = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            let body_trimmed = body.trim();
            // Only save if there's content
            if !to.is_empty() || !subject.is_empty() || (!body_trimmed.is_empty() && !body_trimmed.starts_with("-- \n")) {
                let d = crate::mail::drafts::Draft {
                    id: draft_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    to,
                    cc: cc_d.text().to_string(),
                    bcc: bcc_d.text().to_string(),
                    subject,
                    body,
                    timestamp: chrono::Utc::now().timestamp(),
                };
                crate::mail::drafts::save_draft(&d).ok();
            }
            glib::Propagation::Proceed
        });
    }

    // Attach button
    {
        let atts = attachments.clone();
        let area = attach_area.clone();
        let win = window.clone();
        attach_btn.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::new();
            let atts = atts.clone();
            let area = area.clone();
            dialog.open(
                Some(&win),
                gtk::gio::Cancellable::NONE,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let filename = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            if let Ok(data) = std::fs::read(&path) {
                                atts.borrow_mut().push((filename.clone(), data));
                                let chip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                                let lbl = gtk::Label::new(Some(&filename));
                                lbl.add_css_class("caption");
                                chip.append(&lbl);
                                let remove_btn =
                                    gtk::Button::from_icon_name("window-close-symbolic");
                                remove_btn.add_css_class("flat");
                                remove_btn.add_css_class("circular");
                                let atts2 = atts.clone();
                                let fname = filename.clone();
                                let area2 = area.clone();
                                remove_btn.connect_clicked(move |_| {
                                    atts2.borrow_mut().retain(|(n, _)| n != &fname);
                                    // Remove the chip's parent FlowBoxChild
                                    if area2.observe_children().n_items() == 0 {
                                        area2.set_visible(false);
                                    }
                                });
                                chip.append(&remove_btn);
                                area.append(&chip);
                                area.set_visible(true);
                            }
                        }
                    }
                },
            );
        });
    }

    // Bold button — toggle tag on selection
    {
        let bv = body_view.clone();
        bold_btn.connect_clicked(move |_| {
            toggle_tag(&bv, "bold");
        });
    }

    // Italic button
    {
        let bv = body_view.clone();
        italic_btn.connect_clicked(move |_| {
            toggle_tag(&bv, "italic");
        });
    }

    // Link button
    {
        let bv = body_view.clone();
        let win2 = window.clone();
        link_btn.connect_clicked(move |_| {
            let buffer = bv.buffer();
            let Some((start, end)) = buffer.selection_bounds() else { return };
            let text = buffer.text(&start, &end, false).to_string();
            if text.is_empty() { return; }

            let dialog = adw::Window::builder()
                .title(t("compose.link"))
                .default_width(400)
                .default_height(150)
                .transient_for(&win2)
                .modal(true)
                .build();
            let dbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let dheader = adw::HeaderBar::new();
            let ok_btn = gtk::Button::with_label("OK");
            ok_btn.add_css_class("suggested-action");
            dheader.pack_end(&ok_btn);
            dbox.append(&dheader);
            let url_entry = gtk::Entry::new();
            url_entry.set_placeholder_text(Some("https://"));
            url_entry.set_margin_start(16);
            url_entry.set_margin_end(16);
            url_entry.set_margin_top(16);
            url_entry.set_margin_bottom(16);
            dbox.append(&url_entry);
            dialog.set_content(Some(&dbox));

            let bv2 = bv.clone();
            let dialog2 = dialog.clone();
            ok_btn.connect_clicked(move |_| {
                let url = url_entry.text().to_string();
                if !url.is_empty() {
                    let buffer = bv2.buffer();
                    if let Some((start, end)) = buffer.selection_bounds() {
                        // Create a unique link tag with URL stored in name
                        let tag_name = format!("link:{url}");
                        if buffer.tag_table().lookup(&tag_name).is_none() {
                            buffer.create_tag(Some(&tag_name), &[
                                ("underline", &gtk::pango::Underline::Single),
                                ("foreground", &"#5294e2"),
                            ]);
                        }
                        buffer.apply_tag_by_name(&tag_name, &start, &end);
                    }
                }
                dialog2.close();
            });
            dialog.present();
        });
    }

    // Send action
    let to_e = to_entry.1.clone();
    let cc_e = cc_entry.1.clone();
    let bcc_e = bcc_entry.1.clone();
    let subj_e = subject_entry.1.clone();
    let body_v = body_view.clone();
    let win = window.clone();
    let toast_ov = toast_overlay.clone();
    let atts = attachments.clone();

    log::info!("Compose window created, presenting...");

    let prefill_draft = draft.clone();
    let prefill_sig = config.signature.clone();

    send_btn.connect_clicked(move |_| {
        let to = to_e.text().to_string();
        let cc = cc_e.text().to_string();
        let bcc = bcc_e.text().to_string();
        let subject = subj_e.text().to_string();
        let buffer = body_v.buffer();
        let body = buffer_to_html(&buffer);

        if to.trim().is_empty() {
            toast_ov.add_toast(adw::Toast::new(t("compose.enter_recipient")));
            return;
        }

        let config = config.clone();
        let win = win.clone();
        let attachments = atts.borrow().clone();
        let sent = sent_flag.clone();
        let imap = imap_client.clone();
        let parent_toast = parent_toast.clone();

        // Close compose window immediately
        sent.set(true);
        win.close();

        // Undo send: 5 second delay, toast on parent window
        let cancelled = Rc::new(std::cell::Cell::new(false));
        let toast = adw::Toast::new(t("toast.sending_undo"));
        toast.set_timeout(5);
        toast.set_button_label(Some(t("toast.undo")));
        {
            let cancelled2 = cancelled.clone();
            let pt = parent_toast.clone();
            let save_to = to.clone();
            let save_cc = cc.clone();
            let save_bcc = bcc.clone();
            let save_subj = subject.clone();
            let save_body = body.clone();
            toast.connect_button_clicked(move |_| {
                cancelled2.set(true);
                let d = crate::mail::drafts::Draft {
                    id: uuid::Uuid::new_v4().to_string(),
                    to: save_to.clone(), cc: save_cc.clone(), bcc: save_bcc.clone(),
                    subject: save_subj.clone(), body: save_body.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                };
                crate::mail::drafts::save_draft(&d).ok();
                pt.add_toast(adw::Toast::new(t("draft.saved")));
            });
        }
        parent_toast.add_toast(toast);

        let cancelled3 = cancelled.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || {
            if cancelled3.get() {
                return;
            }

            // Clone for error queue fallback
            let q_email = config.email.clone();
            let q_to = to.clone();
            let q_cc = cc.clone();
            let q_bcc = bcc.clone();
            let q_subj = subject.clone();
            let q_body = body.clone();

            runtime::spawn_on_main(
                async move {
                    let mut config = config;
                    if config.password.is_empty() {
                        match crate::config::load_password(&config.email).await {
                            Ok(pw) => config.password = pw,
                            Err(e) => return Err(crate::mail::error::MailError::Auth(e)),
                        }
                    }
                    let raw = SmtpClient::send(&config, &to, &cc, &bcc, &subject, &body, &attachments).await?;
                    log::info!("SMTP send OK, raw message size: {} bytes", raw.len());
                    let addrs: Vec<&str> = to.split(',').chain(cc.split(',')).chain(bcc.split(','))
                        .map(str::trim).filter(|s| !s.is_empty()).collect();
                    crate::mail::contacts::add_addresses(&addrs);
                    if let Some(ref client) = imap {
                        if let Ok(Some(sent_folder)) = client.find_sent_folder().await {
                            client.append_to_folder(&sent_folder, &raw).await.ok();
                        }
                    }
                    Ok(())
                },
                move |result| {
                    if let Err(e) = result {
                        crate::mail::send_queue::enqueue(crate::mail::send_queue::QueuedMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            account_email: q_email,
                            to: q_to, cc: q_cc, bcc: q_bcc,
                            subject: q_subj, body: q_body,
                            attachments: vec![],
                        });
                        parent_toast.add_toast(adw::Toast::new(&tf("compose.send_failed", &[&e.to_string()])));
                    }
                },
            );
        });
    });

    window.present();

    // Pre-fill after present (so autocomplete popover can attach to realized entry)
    if let Some(ref d) = prefill_draft {
        to_entry.1.set_text(&d.to);
        cc_entry.1.set_text(&d.cc);
        bcc_entry.1.set_text(&d.bcc);
        subject_entry.1.set_text(&d.subject);
        body_view.buffer().set_text(&d.body);
    } else if !prefill_sig.is_empty() {
        body_view.buffer().set_text(&format!("\n\n-- \n{prefill_sig}"));
    }
}

pub fn show_reply_window(
    parent: &impl IsA<gtk::Window>,
    config: AccountConfig,
    reply_to: &str,
    subject: &str,
    original_body: &str,
    imap_client: Option<Arc<ImapClient>>,
    parent_toast: adw::ToastOverlay,
    orig_message_id: &str,
    orig_references: &str,
) {
    let window = adw::Window::builder()
        .title(t("compose.reply_title"))
        .default_width(640)
        .default_height(500)
        .transient_for(parent)
        .modal(true)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    let send_btn = gtk::Button::with_label(t("compose.send"));
    send_btn.add_css_class("suggested-action");
    header.pack_end(&send_btn);
    main_box.append(&header);

    let form_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    form_box.set_margin_start(12);
    form_box.set_margin_end(12);
    form_box.set_margin_top(8);

    let to_entry = create_field_row(t("compose.to"));
    to_entry.1.set_text(reply_to);
    form_box.append(&to_entry.0);

    let re_subject = if subject.starts_with("Re:") {
        subject.to_string()
    } else {
        tf("compose.re_subject", &[subject])
    };
    let subject_entry = create_field_row(t("compose.subject"));
    subject_entry.1.set_text(&re_subject);
    form_box.append(&subject_entry.0);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.set_margin_top(8);
    form_box.append(&separator);

    let body_view = gtk::TextView::new();
    body_view.set_wrap_mode(gtk::WrapMode::Word);
    body_view.set_left_margin(8);
    body_view.set_right_margin(8);
    body_view.set_top_margin(8);

    let quoted = original_body
        .lines()
        .map(|l| format!("> {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    let sig = if config.signature.is_empty() {
        String::new()
    } else {
        format!("\n\n-- \n{}", config.signature)
    };
    body_view.buffer().set_text(&format!("{sig}\n\n{quoted}"));

    let body_scroll = gtk::ScrolledWindow::new();
    body_scroll.set_child(Some(&body_view));
    body_scroll.set_vexpand(true);
    form_box.append(&body_scroll);

    main_box.append(&form_box);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&main_box));
    window.set_content(Some(&toast_overlay));

    let to_e = to_entry.1.clone();
    let subj_e = subject_entry.1.clone();
    let body_v = body_view.clone();
    let win = window.clone();
    let reply_msg_id = orig_message_id.to_string();
    let reply_refs = orig_references.to_string();

    send_btn.connect_clicked(move |_| {
        let to = to_e.text().to_string();
        let subject = subj_e.text().to_string();
        let buffer = body_v.buffer();
        let body = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        let config = config.clone();
        let win = win.clone();
        let imap = imap_client.clone();
        let parent_toast = parent_toast.clone();

        // Close reply window immediately
        win.close();

        // Undo send: 5 second delay on parent
        let cancelled = Rc::new(std::cell::Cell::new(false));
        let toast = adw::Toast::new(t("toast.sending_undo"));
        toast.set_timeout(5);
        toast.set_button_label(Some(t("toast.undo")));
        {
            let cancelled2 = cancelled.clone();
            let pt = parent_toast.clone();
            let save_to = to.clone();
            let save_subj = subject.clone();
            let save_body = body.clone();
            toast.connect_button_clicked(move |_| {
                cancelled2.set(true);
                let d = crate::mail::drafts::Draft {
                    id: uuid::Uuid::new_v4().to_string(),
                    to: save_to.clone(), cc: String::new(), bcc: String::new(),
                    subject: save_subj.clone(), body: save_body.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                };
                crate::mail::drafts::save_draft(&d).ok();
                pt.add_toast(adw::Toast::new(t("draft.saved")));
            });
        }
        parent_toast.add_toast(toast);

        let cancelled3 = cancelled.clone();
        let reply_msg_id = reply_msg_id.clone();
        let reply_refs = reply_refs.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || {
            if cancelled3.get() {
                return;
            }
            runtime::spawn_on_main(
                async move {
                    let mut config = config;
                    if config.password.is_empty() {
                        match crate::config::load_password(&config.email).await {
                            Ok(pw) => config.password = pw,
                            Err(e) => return Err(crate::mail::error::MailError::Auth(e)),
                        }
                    }
                    let raw = SmtpClient::send_threaded(&config, &to, "", "", &subject, &body, &[], &reply_msg_id.clone(), &reply_refs.clone()).await?;
                    if let Some(ref client) = imap {
                        if let Ok(Some(sent_folder)) = client.find_sent_folder().await {
                            client.append_to_folder(&sent_folder, &raw).await.ok();
                        }
                    }
                    Ok(())
                },
                move |result| {
                    if let Err(e) = result {
                        parent_toast.add_toast(adw::Toast::new(&tf("compose.send_failed", &[&e.to_string()])));
                    }
                },
            );
        });
    });

    window.present();
}

fn create_field_row(label_text: &str) -> (gtk::Box, gtk::Entry) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_margin_top(4);
    row.set_margin_bottom(4);

    let label = gtk::Label::new(Some(label_text));
    label.set_width_chars(8);
    label.set_halign(gtk::Align::End);
    label.add_css_class("dim-label");
    row.append(&label);

    let entry = gtk::Entry::new();
    entry.set_hexpand(true);
    row.append(&entry);

    // Contact autocomplete — popover created lazily on first use
    let popover: Rc<std::cell::RefCell<Option<gtk::Popover>>> = Rc::new(std::cell::RefCell::new(None));
    let entry2 = entry.clone();
    let pop_ref = popover.clone();
    entry.connect_changed(move |e| {
        let text = e.text().to_string();
        let query = text.rsplit(',').next().unwrap_or("").trim();
        if query.len() < 2 {
            if let Some(ref p) = *pop_ref.borrow() {
                p.popdown();
            }
            return;
        }
        let results = crate::mail::contacts::search(query);
        if results.is_empty() {
            if let Some(ref p) = *pop_ref.borrow() {
                p.popdown();
            }
            return;
        }
        // Create popover lazily (entry must be in window by now)
        let mut pop_opt = pop_ref.borrow_mut();
        let pop = pop_opt.get_or_insert_with(|| {
            let p = gtk::Popover::new();
            p.set_parent(&entry2);
            p.set_autohide(false);
            p.set_has_arrow(false);
            p
        });
        // Rebuild suggestions
        let suggestions_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        for addr in results {
            let btn = gtk::Button::with_label(&addr);
            btn.add_css_class("flat");
            btn.set_halign(gtk::Align::Start);
            let e2 = entry2.clone();
            let p2 = pop.clone();
            let a = addr.clone();
            btn.connect_clicked(move |_| {
                let current = e2.text().to_string();
                let prefix: String = current.rsplit_once(',')
                    .map(|(before, _)| format!("{before}, "))
                    .unwrap_or_default();
                e2.set_text(&format!("{prefix}{a}"));
                e2.set_position(-1);
                p2.popdown();
            });
            suggestions_box.append(&btn);
        }
        pop.set_child(Some(&suggestions_box));
        pop.popup();
    });

    (row, entry)
}

pub fn show_reply_all_window(
    parent: &impl IsA<gtk::Window>,
    config: AccountConfig,
    reply_to: &str,
    cc: &str,
    subject: &str,
    original_body: &str,
    imap_client: Option<Arc<ImapClient>>,
    parent_toast: adw::ToastOverlay,
) {
    // Reuse show_compose_window with a pre-built draft
    let re_subject = if subject.starts_with("Re:") {
        subject.to_string()
    } else {
        tf("compose.re_subject", &[subject])
    };
    let quoted = original_body
        .lines()
        .map(|l| format!("> {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    let sig = if config.signature.is_empty() {
        String::new()
    } else {
        format!("\n\n-- \n{}", config.signature)
    };
    let body = format!("{sig}\n\n{quoted}");

    let draft = crate::mail::drafts::Draft {
        id: String::new(),
        to: reply_to.to_string(),
        cc: cc.to_string(),
        bcc: String::new(),
        subject: re_subject,
        body,
        timestamp: 0,
    };
    show_compose_window(parent, config, Some(draft), imap_client, parent_toast);
}

pub fn show_forward_window(
    parent: &impl IsA<gtk::Window>,
    config: AccountConfig,
    original: &crate::mail::models::MailMessage,
    imap_client: Option<Arc<ImapClient>>,
    parent_toast: adw::ToastOverlay,
) {
    let fwd_subject = if original.subject.starts_with("Fwd:") {
        original.subject.clone()
    } else {
        tf("compose.fwd_subject", &[&original.subject])
    };
    let from_str = original.from_display();
    let to_str: String = original.to.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
    let date_str = original.date_display();
    let body_text = original.body_text.as_deref().unwrap_or("");

    let sig = if config.signature.is_empty() {
        String::new()
    } else {
        format!("\n\n-- \n{}", config.signature)
    };
    let body = format!(
        "{sig}\n\n---------- Forwarded message ----------\nFrom: {from_str}\nDate: {date_str}\nSubject: {}\nTo: {to_str}\n\n{body_text}",
        original.subject
    );

    // Pre-load original attachments
    let draft = crate::mail::drafts::Draft {
        id: String::new(),
        to: String::new(),
        cc: String::new(),
        bcc: String::new(),
        subject: fwd_subject,
        body,
        timestamp: 0,
    };
    // TODO: forward attachments (need to extend Draft or compose_window to accept them)
    show_compose_window(parent, config, Some(draft), imap_client, parent_toast);
}

fn toggle_tag(text_view: &gtk::TextView, tag_name: &str) {
    let buffer = text_view.buffer();
    let Some((start, end)) = buffer.selection_bounds() else { return };
    // Check if tag is already applied at the start of selection
    let has_tag = start.tags().iter().any(|t| {
        t.name().map(|n| n.as_str() == tag_name).unwrap_or(false)
    });
    if has_tag {
        buffer.remove_tag_by_name(tag_name, &start, &end);
    } else {
        buffer.apply_tag_by_name(tag_name, &start, &end);
    }
}

/// Extract formatting state at a given iterator position.
fn get_format_state(iter: &gtk::TextIter) -> (bool, bool, Option<String>) {
    let mut bold = false;
    let mut italic = false;
    let mut link_url: Option<String> = None;
    for tag in iter.tags() {
        if let Some(name) = tag.name() {
            match name.as_str() {
                "bold" => bold = true,
                "italic" => italic = true,
                n if n.starts_with("link:") => {
                    link_url = Some(n[5..].to_string());
                }
                _ => {}
            }
        }
    }
    (bold, italic, link_url)
}

/// Convert a TextBuffer with tags to HTML string for sending.
fn buffer_to_html(buffer: &gtk::TextBuffer) -> String {
    let mut html = String::new();
    let mut iter = buffer.start_iter();
    let end = buffer.end_iter();

    let mut cur_bold = false;
    let mut cur_italic = false;
    let mut cur_link: Option<String> = None;
    let mut run_text = String::new();

    while iter < end {
        let (bold, italic, link) = get_format_state(&iter);
        let ch = iter.char();

        // If formatting changed, flush the current run
        if bold != cur_bold || italic != cur_italic || link != cur_link {
            if !run_text.is_empty() {
                html.push_str(&wrap_run(&run_text, cur_bold, cur_italic, &cur_link));
                run_text.clear();
            }
            cur_bold = bold;
            cur_italic = italic;
            cur_link = link;
        }

        match ch {
            '<' => run_text.push_str("&lt;"),
            '>' => run_text.push_str("&gt;"),
            '&' => run_text.push_str("&amp;"),
            '\n' => run_text.push_str("<br>"),
            c => run_text.push(c),
        }

        iter.forward_char();
    }

    // Flush remaining run
    if !run_text.is_empty() {
        html.push_str(&wrap_run(&run_text, cur_bold, cur_italic, &cur_link));
    }

    html
}

fn wrap_run(text: &str, bold: bool, italic: bool, link: &Option<String>) -> String {
    let mut result = text.to_string();
    if bold { result = format!("<b>{result}</b>"); }
    if italic { result = format!("<i>{result}</i>"); }
    if let Some(url) = link {
        result = format!("<a href=\"{url}\">{result}</a>");
    }
    result
}
