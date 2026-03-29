use crate::i18n::{t, tf};
use crate::mail::models::MailMessage;
use adw::prelude::*;
use gtk::glib;
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
    current_html: std::rc::Rc<std::cell::RefCell<Option<String>>>,
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

        let settings = webkit6::prelude::WebViewExt::settings(&web_view).unwrap();
        settings.set_auto_load_images(false);
        settings.set_enable_javascript(false);
        settings.set_allow_modal_dialogs(false);

        web_view.set_background_color(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

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
                let settings = webkit6::prelude::WebViewExt::settings(&wv).unwrap();
                settings.set_auto_load_images(true);
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
        let settings = webkit6::prelude::WebViewExt::settings(&self.web_view).unwrap();
        settings.set_auto_load_images(false);
        self.load_images_btn.set_visible(msg.body_html.is_some());
        *self.current_html.borrow_mut() = None;

        let html = if let Some(ref html_body) = msg.body_html {
            let sanitized = sanitize_html(html_body);
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
    color: #e0e0e0;
    background: transparent;
    word-wrap: break-word;
    overflow-wrap: break-word;
}}
@media (prefers-color-scheme: light) {{
    body {{ color: #1a1a1a; }}
}}
img {{ max-width: 100%; height: auto; }}
a {{ color: #5294e2; cursor: pointer; }}
blockquote {{
    border-left: 3px solid #555;
    margin: 8px 0;
    padding: 4px 12px;
    color: #aaa;
}}
</style></head><body>{body}</body></html>"#
    )
}

fn sanitize_html(html: &str) -> String {
    ammonia::Builder::default()
        .add_tags(&[
            "h1", "h2", "h3", "h4", "h5", "h6", "p", "br", "hr", "div", "span",
            "b", "i", "u", "strong", "em", "a", "ul", "ol", "li", "blockquote",
            "pre", "code", "table", "thead", "tbody", "tr", "td", "th", "img",
            "sub", "sup", "dl", "dt", "dd", "center", "font", "small", "big",
        ])
        .add_tag_attributes("a", &["href"])
        .add_tag_attributes("img", &["src", "alt", "width", "height"])
        .add_tag_attributes("td", &["colspan", "rowspan", "align", "valign"])
        .add_tag_attributes("th", &["colspan", "rowspan", "align", "valign"])
        .add_tag_attributes("font", &["color", "size", "face"])
        .add_tag_attributes("div", &["align"])
        .add_tag_attributes("p", &["align"])
        .url_schemes(["http", "https", "cid", "data"].iter().copied().collect())
        .clean(html)
        .to_string()
}

fn plain_to_html(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "<br>");
    wrap_html(&escaped)
}
