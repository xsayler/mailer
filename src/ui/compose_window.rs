use crate::config::AccountConfig;
use crate::i18n::{t, tf};
use crate::mail::smtp_client::SmtpClient;
use crate::runtime;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

pub fn show_compose_window(parent: &impl IsA<gtk::Window>, config: AccountConfig, draft: Option<crate::mail::drafts::Draft>) {
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

    // Body
    let body_view = gtk::TextView::new();
    body_view.set_wrap_mode(gtk::WrapMode::Word);
    body_view.set_left_margin(8);
    body_view.set_right_margin(8);
    body_view.set_top_margin(8);
    body_view.set_bottom_margin(8);

    let body_scroll = gtk::ScrolledWindow::new();
    body_scroll.set_child(Some(&body_view));
    body_scroll.set_vexpand(true);

    // Pre-fill from draft or signature
    if let Some(ref d) = draft {
        to_entry.1.set_text(&d.to);
        cc_entry.1.set_text(&d.cc);
        bcc_entry.1.set_text(&d.bcc);
        subject_entry.1.set_text(&d.subject);
        body_view.buffer().set_text(&d.body);
    } else if !config.signature.is_empty() {
        body_view.buffer().set_text(&format!("\n\n-- \n{}", config.signature));
    }

    form_box.append(&body_scroll);
    main_box.append(&form_box);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&main_box));
    window.set_content(Some(&toast_overlay));

    // Attachments storage
    let attachments: Rc<RefCell<Vec<(String, Vec<u8>)>>> = Rc::new(RefCell::new(Vec::new()));
    let sent_flag = Rc::new(std::cell::Cell::new(false));

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

    // Bold button
    {
        let bv = body_view.clone();
        bold_btn.connect_clicked(move |_| {
            wrap_selection(&bv, "<b>", "</b>");
        });
    }

    // Italic button
    {
        let bv = body_view.clone();
        italic_btn.connect_clicked(move |_| {
            wrap_selection(&bv, "<i>", "</i>");
        });
    }

    // Link button
    {
        let bv = body_view.clone();
        let win2 = window.clone();
        link_btn.connect_clicked(move |_| {
            let buffer = bv.buffer();
            let text = if let Some((start, end)) = buffer.selection_bounds() {
                buffer.text(&start, &end, false).to_string()
            } else {
                String::new()
            };
            // Simple approach: wrap selected text as link
            if !text.is_empty() {
                // Prompt for URL - use a simple dialog
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
                let text2 = text.clone();
                let dialog2 = dialog.clone();
                ok_btn.connect_clicked(move |_| {
                    let url = url_entry.text().to_string();
                    if !url.is_empty() {
                        let link = format!("<a href=\"{url}\">{text2}</a>");
                        let buffer = bv2.buffer();
                        if let Some((mut start, mut end)) = buffer.selection_bounds() {
                            buffer.delete(&mut start, &mut end);
                            buffer.insert(&mut start, &link);
                        }
                    }
                    dialog2.close();
                });
                dialog.present();
            }
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

    send_btn.connect_clicked(move |btn| {
        let to = to_e.text().to_string();
        let cc = cc_e.text().to_string();
        let bcc = bcc_e.text().to_string();
        let subject = subj_e.text().to_string();
        let buffer = body_v.buffer();
        let body = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        if to.trim().is_empty() {
            toast_ov.add_toast(adw::Toast::new(t("compose.enter_recipient")));
            return;
        }

        btn.set_sensitive(false);
        btn.set_label(t("compose.sending"));

        let config = config.clone();
        let win = win.clone();
        let btn = btn.clone();
        let toast_ov = toast_ov.clone();
        let attachments = atts.borrow().clone();
        let sent = sent_flag.clone();

        runtime::spawn_on_main(
            async move {
                SmtpClient::send(&config, &to, &cc, &bcc, &subject, &body, &attachments).await
            },
            move |result| match result {
                Ok(()) => {
                    sent.set(true);
                    win.close();
                }
                Err(e) => {
                    btn.set_sensitive(true);
                    btn.set_label(t("compose.send"));
                    toast_ov.add_toast(adw::Toast::new(&tf("compose.send_failed", &[&e.to_string()])));
                }
            },
        );
    });

    window.present();
}

pub fn show_reply_window(
    parent: &impl IsA<gtk::Window>,
    config: AccountConfig,
    reply_to: &str,
    subject: &str,
    original_body: &str,
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
    let toast_ov = toast_overlay.clone();

    send_btn.connect_clicked(move |btn| {
        let to = to_e.text().to_string();
        let subject = subj_e.text().to_string();
        let buffer = body_v.buffer();
        let body = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        btn.set_sensitive(false);
        btn.set_label(t("compose.sending"));

        let config = config.clone();
        let win = win.clone();
        let btn = btn.clone();
        let toast_ov = toast_ov.clone();

        runtime::spawn_on_main(
            async move { SmtpClient::send(&config, &to, "", "", &subject, &body, &[]).await },
            move |result| match result {
                Ok(()) => win.close(),
                Err(e) => {
                    btn.set_sensitive(true);
                    btn.set_label(t("compose.send"));
                    toast_ov.add_toast(adw::Toast::new(&tf("compose.send_failed", &[&e.to_string()])));
                }
            },
        );
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

    (row, entry)
}

fn wrap_selection(text_view: &gtk::TextView, prefix: &str, suffix: &str) {
    let buffer = text_view.buffer();
    if let Some((mut start, mut end)) = buffer.selection_bounds() {
        let selected = buffer.text(&start, &end, false).to_string();
        let wrapped = format!("{prefix}{selected}{suffix}");
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, &wrapped);
    }
}
