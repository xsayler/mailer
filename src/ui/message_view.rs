use crate::i18n::{t, tf};
use crate::mail::models::MailMessage;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;

pub struct MessageView {
    pub widget: gtk::Box,
    from_label: gtk::Label,
    to_label: gtk::Label,
    cc_label: gtk::Label,
    subject_label: gtk::Label,
    date_label: gtk::Label,
    web_view: webkit6::WebView,
    attachment_box: gtk::Box,
    load_images_btn: gtk::Button,
    current_html: Rc<RefCell<Option<String>>>,
    on_quick_reply: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
    stack: gtk::Stack,
}

impl MessageView {
    pub fn new() -> Self {
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let stack = gtk::Stack::new();

        // Empty state
        let empty_status = adw::StatusPage::new();
        empty_status.set_icon_name(Some("mail-read-symbolic"));
        empty_status.set_title(t("message.no_selected"));
        empty_status.set_description(Some(t("message.select_hint")));
        stack.add_named(&empty_status, Some("empty"));

        // Message view
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Header
        let header_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        header_box.set_margin_start(16);
        header_box.set_margin_end(16);
        header_box.set_margin_top(12);
        header_box.set_margin_bottom(12);

        let subject_label = gtk::Label::new(None);
        subject_label.set_halign(gtk::Align::Start);
        subject_label.set_wrap(true);
        subject_label.add_css_class("title-2");
        header_box.append(&subject_label);

        let from_label = gtk::Label::new(None);
        from_label.set_halign(gtk::Align::Start);
        from_label.set_wrap(true);
        from_label.add_css_class("heading");
        header_box.append(&from_label);

        let to_label = gtk::Label::new(None);
        to_label.set_halign(gtk::Align::Start);
        to_label.set_wrap(true);
        to_label.add_css_class("dim-label");
        header_box.append(&to_label);

        let cc_label = gtk::Label::new(None);
        cc_label.set_halign(gtk::Align::Start);
        cc_label.set_wrap(true);
        cc_label.add_css_class("dim-label");
        cc_label.set_visible(false);
        header_box.append(&cc_label);

        let date_label = gtk::Label::new(None);
        date_label.set_halign(gtk::Align::Start);
        date_label.add_css_class("dim-label");
        date_label.add_css_class("caption");
        header_box.append(&date_label);

        content_box.append(&header_box);

        // Attachment bar
        let attachment_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        attachment_box.set_margin_start(16);
        attachment_box.set_margin_end(16);
        attachment_box.set_margin_bottom(8);
        attachment_box.set_visible(false);
        content_box.append(&attachment_box);

        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);

        // WebView
        let web_view = webkit6::WebView::new();
        web_view.set_vexpand(true);
        web_view.set_hexpand(true);

        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&web_view) {
            settings.set_auto_load_images(false);
            settings.set_enable_javascript(false);
            settings.set_allow_modal_dialogs(false);
        }

        // Let the HTML CSS handle colors (light/dark via prefers-color-scheme)

        // Load images button
        let load_images_btn = gtk::Button::with_label(t("message.load_images"));
        load_images_btn.add_css_class("flat");
        load_images_btn.set_margin_start(16);
        load_images_btn.set_margin_end(16);
        load_images_btn.set_margin_top(4);
        load_images_btn.set_margin_bottom(4);
        load_images_btn.set_visible(false);

        let current_html: std::rc::Rc<std::cell::RefCell<Option<String>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));

        {
            let wv = web_view.clone();
            let btn = load_images_btn.clone();
            let html_ref = current_html.clone();
            load_images_btn.connect_clicked(move |_| {
                if let Some(settings) = webkit6::prelude::WebViewExt::settings(&wv) {
                    settings.set_auto_load_images(true);
                }
                if let Some(ref html) = *html_ref.borrow() {
                    wv.load_html(html, None);
                }
                btn.set_visible(false);
            });
        }

        web_view.connect_decide_policy(|_, decision, decision_type| {
            match decision_type {
                webkit6::PolicyDecisionType::NavigationAction
                | webkit6::PolicyDecisionType::NewWindowAction => {
                    if let Some(nav_decision) =
                        decision.downcast_ref::<webkit6::NavigationPolicyDecision>()
                    {
                        if let Some(mut action) = nav_decision.navigation_action() {
                            let nav_type = action.navigation_type();
                            if nav_type != webkit6::NavigationType::Other {
                                if let Some(request) = action.request() {
                                    if let Some(uri) = request.uri() {
                                        let uri_str = uri.to_string();
                                        if uri_str.starts_with("http://")
                                            || uri_str.starts_with("https://")
                                            || uri_str.starts_with("mailto:")
                                        {
                                            let _ = gtk::gio::AppInfo::launch_default_for_uri(
                                                &uri_str,
                                                gtk::gio::AppLaunchContext::NONE,
                                            );
                                        }
                                    }
                                }
                                decision.ignore();
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
            false
        });

        content_box.append(&load_images_btn);
        content_box.append(&separator);
        content_box.append(&web_view);

        // Quick reply panel
        let reply_sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        content_box.append(&reply_sep);

        let reply_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        reply_box.set_margin_start(12);
        reply_box.set_margin_end(12);
        reply_box.set_margin_top(6);
        reply_box.set_margin_bottom(6);

        let reply_entry = gtk::Entry::new();
        reply_entry.set_placeholder_text(Some(t("compose.quick_reply")));
        reply_entry.set_hexpand(true);
        reply_box.append(&reply_entry);

        let reply_send_btn = gtk::Button::from_icon_name("mail-send-symbolic");
        reply_send_btn.add_css_class("suggested-action");
        reply_send_btn.add_css_class("circular");
        reply_box.append(&reply_send_btn);

        content_box.append(&reply_box);

        let on_quick_reply: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));

        {
            let entry = reply_entry.clone();
            let oqr = on_quick_reply.clone();
            reply_send_btn.connect_clicked(move |_| {
                let text = entry.text().to_string();
                if text.trim().is_empty() { return; }
                if let Some(ref cb) = *oqr.borrow() {
                    cb(text);
                }
                entry.set_text("");
            });
        }
        {
            let entry = reply_entry.clone();
            let oqr = on_quick_reply.clone();
            reply_entry.connect_activate(move |_| {
                let text = entry.text().to_string();
                if text.trim().is_empty() { return; }
                if let Some(ref cb) = *oqr.borrow() {
                    cb(text);
                }
                entry.set_text("");
            });
        }

        stack.add_named(&content_box, Some("message"));
        stack.set_visible_child_name("empty");

        main_box.append(&stack);
        main_box.set_hexpand(true);
        main_box.set_vexpand(true);

        Self {
            widget: main_box,
            from_label,
            to_label,
            cc_label,
            subject_label,
            date_label,
            web_view,
            attachment_box,
            load_images_btn,
            current_html,
            on_quick_reply,
            stack,
        }
    }

    pub fn show_message(&self, msg: &MailMessage) {
        self.subject_label.set_text(&msg.subject);
        self.from_label
            .set_text(&tf("message.from", &[&msg.from_display()]));

        let to_str: String = msg
            .to
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        self.to_label.set_text(&tf("message.to", &[&to_str]));

        if !msg.cc.is_empty() {
            let cc_str: String = msg.cc.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
            self.cc_label.set_text(&tf("message.cc", &[&cc_str]));
            self.cc_label.set_visible(true);
        } else {
            self.cc_label.set_visible(false);
        }

        self.date_label.set_text(&msg.date_display());

        // Unsubscribe button
        if let Some(ref unsub) = msg.list_unsubscribe {
            // Extract URL from header (may be wrapped in < >)
            let url = unsub.trim().trim_matches(|c| c == '<' || c == '>').to_string();
            if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
                let unsub_btn = gtk::Button::with_label(t("message.unsubscribe"));
                unsub_btn.add_css_class("flat");
                unsub_btn.add_css_class("caption");
                unsub_btn.add_css_class("error");
                unsub_btn.connect_clicked(move |_| {
                    let _ = gtk::gio::AppInfo::launch_default_for_uri(&url, gtk::gio::AppLaunchContext::NONE);
                });
                self.attachment_box.append(&unsub_btn);
                self.attachment_box.set_visible(true);
            }
        }

        // Attachments
        // Clear old attachment buttons
        while let Some(child) = self.attachment_box.first_child() {
            self.attachment_box.remove(&child);
        }

        if !msg.attachments.is_empty() {
            let icon = gtk::Image::from_icon_name("mail-attachment-symbolic");
            self.attachment_box.append(&icon);

            let label = gtk::Label::new(Some(t("attachment.title")));
            label.add_css_class("dim-label");
            label.add_css_class("caption");
            self.attachment_box.append(&label);

            for att in &msg.attachments {
                // Image preview thumbnail
                if att.content_type.starts_with("image/") && !att.data.is_empty() {
                    let bytes = glib::Bytes::from(&att.data);
                    if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                        let picture = gtk::Picture::for_paintable(&texture);
                        picture.set_size_request(48, 48);
                        picture.set_content_fit(gtk::ContentFit::Contain);
                        self.attachment_box.append(&picture);
                    }
                }

                let size_str = if att.size >= 1_048_576 {
                    tf("attachment.size_mb", &[&format!("{:.1}", att.size as f64 / 1_048_576.0)])
                } else {
                    tf("attachment.size_kb", &[&format!("{}", att.size / 1024)])
                };
                let btn_label = format!("{} ({})", att.filename, size_str);
                let btn = gtk::Button::with_label(&btn_label);
                btn.add_css_class("flat");
                btn.add_css_class("caption");

                let data = att.data.clone();
                let filename = att.filename.clone();
                btn.connect_clicked(move |button| {
                    let dialog = gtk::FileDialog::builder()
                        .initial_name(&filename)
                        .build();

                    let data = data.clone();
                    let button = button.clone();
                    dialog.save(
                        button.root().and_downcast_ref::<gtk::Window>(),
                        gtk::gio::Cancellable::NONE,
                        move |result| {
                            if let Ok(file) = result {
                                if let Some(path) = file.path() {
                                    if let Err(e) = std::fs::write(&path, &data) {
                                        log::error!("Failed to save attachment: {e}");
                                    }
                                }
                            }
                        },
                    );
                });

                self.attachment_box.append(&btn);
            }
            self.attachment_box.set_visible(true);
        } else {
            self.attachment_box.set_visible(false);
        }

        // Reset image blocking for new message
        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&self.web_view) {
            settings.set_auto_load_images(false);
        }
        self.load_images_btn.set_visible(msg.body_html.is_some());
        *self.current_html.borrow_mut() = None;

        let html = if let Some(ref html_body) = msg.body_html {
            // Replace CID references with inline data URIs
            let mut resolved = html_body.clone();
            for att in &msg.attachments {
                if let Some(ref cid) = att.content_id {
                    if att.content_type.starts_with("image/") && !att.data.is_empty() {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&att.data);
                        let data_uri = format!("data:{};base64,{}", att.content_type, b64);
                        let cid_clean = cid.trim_matches(|c| c == '<' || c == '>');
                        resolved = resolved.replace(&format!("cid:{cid_clean}"), &data_uri);
                    }
                }
            }
            let sanitized = sanitize_html(&resolved);
            let wrapped = wrap_html(&sanitized);
            *self.current_html.borrow_mut() = Some(wrapped.clone());
            wrapped
        } else if let Some(ref text_body) = msg.body_text {
            plain_to_html(text_body)
        } else {
            plain_to_html(t("message.no_content"))
        };

        self.web_view.load_html(&html, None);
        self.stack.set_visible_child_name("message");
    }

    /// Show a thread of related messages in a single view.
    pub fn show_thread(&self, messages: &[MailMessage]) {
        if messages.is_empty() { return; }
        if messages.len() == 1 {
            self.show_message(&messages[0]);
            return;
        }

        let first = &messages[0];
        self.subject_label.set_text(&first.subject);
        self.from_label.set_text(&tf("message.from", &[&first.from_display()]));
        self.to_label.set_text("");
        self.cc_label.set_visible(false);
        self.date_label.set_text(&format!("{} messages", messages.len()));
        while let Some(child) = self.attachment_box.first_child() {
            self.attachment_box.remove(&child);
        }
        self.attachment_box.set_visible(false);
        self.load_images_btn.set_visible(false);

        let mut html_parts = Vec::new();
        for msg in messages {
            let from = msg.from_display();
            let date = msg.date_display();
            let body = if let Some(ref h) = msg.body_html {
                sanitize_html(h)
            } else if let Some(ref t) = msg.body_text {
                t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\n', "<br>")
            } else {
                String::new()
            };
            html_parts.push(format!(
                "<div style=\"margin-bottom:16px;padding-bottom:12px;border-bottom:1px solid #444\">\
                <div style=\"font-weight:bold;margin-bottom:4px\">{from}</div>\
                <div style=\"font-size:12px;color:#888;margin-bottom:8px\">{date}</div>\
                <div>{body}</div></div>"
            ));
        }
        let combined = html_parts.join("\n");
        let full_html = wrap_html(&combined);
        *self.current_html.borrow_mut() = Some(full_html.clone());
        self.web_view.load_html(&full_html, None);
        self.stack.set_visible_child_name("message");
    }

    pub fn set_on_quick_reply(&self, cb: impl Fn(String) + 'static) {
        *self.on_quick_reply.borrow_mut() = Some(Box::new(cb));
    }

    pub fn print(&self) {
        let print_op = webkit6::PrintOperation::new(&self.web_view);
        print_op.run_dialog(self.web_view.root().and_then(|r| r.downcast::<gtk::Window>().ok()).as_ref());
    }

    pub fn clear(&self) {
        self.stack.set_visible_child_name("empty");
    }
}

fn wrap_html(body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
body {{
    font-family: system-ui, sans-serif;
    font-size: 14px;
    padding: 12px 16px;
    margin: 0;
    color: #1a1a1a;
    background: #ffffff;
    word-wrap: break-word;
    overflow-wrap: break-word;
}}
img {{ max-width: 100%; height: auto; }}
a {{ color: #1a6fb5; cursor: pointer; }}
blockquote {{
    border-left: 3px solid #ccc;
    margin: 8px 0;
    padding: 4px 12px;
    color: #666;
}}
</style></head><body>{body}</body></html>"#
    )
}

fn sanitize_html(html: &str) -> String {
    let all_tags = &[
        "h1", "h2", "h3", "h4", "h5", "h6", "p", "br", "hr", "div", "span",
        "b", "i", "u", "strong", "em", "a", "ul", "ol", "li", "blockquote",
        "pre", "code", "table", "thead", "tbody", "tfoot", "tr", "td", "th",
        "img", "sub", "sup", "dl", "dt", "dd", "center", "font", "small", "big",
        "caption", "colgroup", "col", "section", "article", "header", "footer",
        "nav", "aside", "figure", "figcaption", "mark", "abbr", "details", "summary",
    ];
    let style_attr = &["style"];
    let mut builder = ammonia::Builder::default();
    builder
        .add_tags(all_tags)
        .add_tag_attributes("a", &["href", "target", "style"])
        .add_tag_attributes("img", &["src", "alt", "width", "height", "style"])
        .add_tag_attributes("td", &["colspan", "rowspan", "align", "valign", "width", "height", "bgcolor", "style"])
        .add_tag_attributes("th", &["colspan", "rowspan", "align", "valign", "width", "height", "bgcolor", "style"])
        .add_tag_attributes("table", &["width", "height", "cellpadding", "cellspacing", "border", "bgcolor", "align", "style"])
        .add_tag_attributes("tr", &["bgcolor", "align", "valign", "style"])
        .add_tag_attributes("font", &["color", "size", "face", "style"])
        .add_tag_attributes("div", &["align", "style"])
        .add_tag_attributes("span", &["style"])
        .add_tag_attributes("p", &["align", "style"])
        .add_tag_attributes("center", &["style"])
        .add_tag_attributes("hr", &["style"])
        .add_tag_attributes("body", &["style"])
        .add_tag_attributes("h1", style_attr).add_tag_attributes("h2", style_attr)
        .add_tag_attributes("h3", style_attr).add_tag_attributes("h4", style_attr)
        .add_tag_attributes("b", style_attr).add_tag_attributes("i", style_attr)
        .add_tag_attributes("strong", style_attr).add_tag_attributes("em", style_attr)
        .add_tag_attributes("ul", style_attr).add_tag_attributes("ol", style_attr)
        .add_tag_attributes("li", style_attr).add_tag_attributes("blockquote", style_attr)
        .url_schemes(["http", "https", "cid", "data"].iter().copied().collect());
    builder.clean(html).to_string()
}

fn plain_to_html(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "<br>");
    wrap_html(&escaped)
}
