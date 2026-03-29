use crate::i18n::t;
use crate::mail::models::{MailFolder, MailMessage, MailMessageObject};
use adw::prelude::*;
use gtk::gio;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Date,
    Sender,
    Subject,
    Unread,
}

pub struct MessageList {
    pub widget: gtk::Box,
    pub list_box: gtk::ListBox,
    pub search_entry: gtk::SearchEntry,
    pub model: gio::ListStore,
    sort_field: Rc<Cell<SortField>>,
    sort_ascending: Rc<Cell<bool>>,
    messages: Rc<std::cell::RefCell<Vec<MailMessage>>>,
    sorted: Rc<std::cell::RefCell<Vec<MailMessage>>>,
    filter_query: Rc<std::cell::RefCell<String>>,
    folders: Rc<std::cell::RefCell<Vec<MailFolder>>>,
    on_server_search: Rc<std::cell::RefCell<Option<Box<dyn Fn(String)>>>>,
    on_load_more: Rc<std::cell::RefCell<Option<Box<dyn Fn()>>>>,
    has_more: Rc<std::cell::Cell<bool>>,
    loading_more: Rc<std::cell::Cell<bool>>,
}

impl MessageList {
    pub fn new() -> Self {
        let model = gio::ListStore::new::<MailMessageObject>();

        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.add_css_class("boxed-list");

        list_box.bind_model(Some(&model), |item| {
            let msg_obj = item.downcast_ref::<MailMessageObject>().unwrap();
            Self::create_message_row(msg_obj)
        });

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_child(Some(&list_box));
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        // Search entry
        let search_entry = gtk::SearchEntry::new();
        search_entry.set_placeholder_text(Some(t("search.placeholder")));
        search_entry.set_margin_start(4);
        search_entry.set_margin_end(4);
        search_entry.set_margin_top(4);

        // Sort bar
        let sort_bar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        sort_bar.set_margin_start(4);
        sort_bar.set_margin_end(4);
        sort_bar.set_margin_top(4);
        sort_bar.set_margin_bottom(4);

        let date_btn = gtk::ToggleButton::with_label(t("sort.date"));
        date_btn.set_active(true);
        date_btn.add_css_class("flat");
        date_btn.add_css_class("caption");

        let sender_btn = gtk::ToggleButton::with_label(t("sort.sender"));
        sender_btn.set_group(Some(&date_btn));
        sender_btn.add_css_class("flat");
        sender_btn.add_css_class("caption");

        let subject_btn = gtk::ToggleButton::with_label(t("sort.subject"));
        subject_btn.set_group(Some(&date_btn));
        subject_btn.add_css_class("flat");
        subject_btn.add_css_class("caption");

        let unread_btn = gtk::ToggleButton::with_label(t("sort.unread"));
        unread_btn.set_group(Some(&date_btn));
        unread_btn.add_css_class("flat");
        unread_btn.add_css_class("caption");

        let dir_btn = gtk::Button::from_icon_name("view-sort-descending-symbolic");
        dir_btn.add_css_class("flat");
        dir_btn.set_tooltip_text(Some("↓"));

        sort_bar.append(&date_btn);
        sort_bar.append(&sender_btn);
        sort_bar.append(&subject_btn);
        sort_bar.append(&unread_btn);
        sort_bar.append(&dir_btn);

        // Main container
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main_box.append(&search_entry);
        main_box.append(&sort_bar);
        main_box.append(&scrolled);

        let sort_field = Rc::new(Cell::new(SortField::Date));
        let sort_ascending = Rc::new(Cell::new(false));
        let messages: Rc<std::cell::RefCell<Vec<MailMessage>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));
        let sorted: Rc<std::cell::RefCell<Vec<MailMessage>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));
        let filter_query: Rc<std::cell::RefCell<String>> =
            Rc::new(std::cell::RefCell::new(String::new()));
        let folders: Rc<std::cell::RefCell<Vec<MailFolder>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));

        let on_server_search: Rc<std::cell::RefCell<Option<Box<dyn Fn(String)>>>> =
            Rc::new(std::cell::RefCell::new(None));
        let on_load_more: Rc<std::cell::RefCell<Option<Box<dyn Fn()>>>> =
            Rc::new(std::cell::RefCell::new(None));
        let has_more = Rc::new(std::cell::Cell::new(false));
        let loading_more = Rc::new(std::cell::Cell::new(false));

        let ml = Self {
            widget: main_box,
            list_box,
            search_entry: search_entry.clone(),
            model,
            sort_field: sort_field.clone(),
            sort_ascending: sort_ascending.clone(),
            messages: messages.clone(),
            sorted: sorted.clone(),
            filter_query: filter_query.clone(),
            folders,
            on_server_search: on_server_search.clone(),
            on_load_more: on_load_more.clone(),
            has_more: has_more.clone(),
            loading_more: loading_more.clone(),
        };

        // Infinite scroll: load more when near bottom
        {
            let hm = has_more.clone();
            let lm = loading_more.clone();
            let olm = on_load_more.clone();
            scrolled.vadjustment().connect_value_changed(move |adj| {
                if !hm.get() || lm.get() {
                    return;
                }
                let value = adj.value();
                let upper = adj.upper();
                let page = adj.page_size();
                // Trigger when within 100px of the bottom
                if value + page + 100.0 >= upper {
                    lm.set(true);
                    if let Some(ref cb) = *olm.borrow() {
                        cb();
                    }
                }
            });
        }

        // Search handler
        {
            let fq = filter_query.clone();
            let msgs = messages.clone();
            let srt = sorted.clone();
            let sf = sort_field.clone();
            let sa = sort_ascending.clone();
            let model = ml.model.clone();
            let lb = ml.list_box.clone();
            let oss = on_server_search.clone();
            search_entry.connect_search_changed(move |entry| {
                let query = entry.text().to_string();
                *fq.borrow_mut() = query.clone();
                if !query.is_empty() {
                    if let Some(ref cb) = *oss.borrow() {
                        cb(query);
                        return;
                    }
                }
                filter_and_sort(&model, &msgs.borrow(), &fq.borrow(), sf.get(), sa.get(), &lb, &srt);
            });
        }

        // Sort button handlers
        {
            let sf = sort_field.clone();
            let sa = sort_ascending.clone();
            let msgs = messages.clone();
            let srt = sorted.clone();
            let fq = filter_query.clone();
            let model = ml.model.clone();
            let lb = ml.list_box.clone();
            date_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    sf.set(SortField::Date);
                    filter_and_sort(&model, &msgs.borrow(), &fq.borrow(), sf.get(), sa.get(), &lb, &srt);
                }
            });
        }
        {
            let sf = sort_field.clone();
            let sa = sort_ascending.clone();
            let msgs = messages.clone();
            let srt = sorted.clone();
            let fq = filter_query.clone();
            let model = ml.model.clone();
            let lb = ml.list_box.clone();
            sender_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    sf.set(SortField::Sender);
                    filter_and_sort(&model, &msgs.borrow(), &fq.borrow(), sf.get(), sa.get(), &lb, &srt);
                }
            });
        }
        {
            let sf = sort_field.clone();
            let sa = sort_ascending.clone();
            let msgs = messages.clone();
            let srt = sorted.clone();
            let fq = filter_query.clone();
            let model = ml.model.clone();
            let lb = ml.list_box.clone();
            subject_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    sf.set(SortField::Subject);
                    filter_and_sort(&model, &msgs.borrow(), &fq.borrow(), sf.get(), sa.get(), &lb, &srt);
                }
            });
        }
        {
            let sf = sort_field.clone();
            let sa = sort_ascending.clone();
            let msgs = messages.clone();
            let srt = sorted.clone();
            let fq = filter_query.clone();
            let model = ml.model.clone();
            let lb = ml.list_box.clone();
            unread_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    sf.set(SortField::Unread);
                    filter_and_sort(&model, &msgs.borrow(), &fq.borrow(), sf.get(), sa.get(), &lb, &srt);
                }
            });
        }
        {
            let sf = sort_field;
            let sa = sort_ascending;
            let msgs = messages;
            let srt = sorted;
            let fq = filter_query;
            let model = ml.model.clone();
            let lb = ml.list_box.clone();
            dir_btn.connect_clicked(move |btn| {
                let new_asc = !sa.get();
                sa.set(new_asc);
                btn.set_icon_name(if new_asc {
                    "view-sort-ascending-symbolic"
                } else {
                    "view-sort-descending-symbolic"
                });
                filter_and_sort(&model, &msgs.borrow(), &fq.borrow(), sf.get(), new_asc, &lb, &srt);
            });
        }

        ml
    }

    pub fn get_sorted_message(&self, index: usize) -> Option<MailMessage> {
        self.sorted.borrow().get(index).cloned()
    }

    pub fn message_count(&self) -> usize {
        self.sorted.borrow().len()
    }

    pub fn set_messages(&self, messages: &[MailMessage]) {
        *self.messages.borrow_mut() = messages.to_vec();
        filter_and_sort(
            &self.model,
            messages,
            &self.filter_query.borrow(),
            self.sort_field.get(),
            self.sort_ascending.get(),
            &self.list_box,
            &self.sorted,
        );
    }

    pub fn append_messages(&self, messages: &[MailMessage]) {
        self.messages.borrow_mut().extend(messages.iter().cloned());
        filter_and_sort(
            &self.model,
            &self.messages.borrow(),
            &self.filter_query.borrow(),
            self.sort_field.get(),
            self.sort_ascending.get(),
            &self.list_box,
            &self.sorted,
        );
    }

    pub fn set_has_more(&self, has_more: bool) {
        self.has_more.set(has_more);
        self.loading_more.set(false);
    }

    pub fn set_on_load_more(&self, cb: impl Fn() + 'static) {
        *self.on_load_more.borrow_mut() = Some(Box::new(cb));
    }

    pub fn set_on_server_search(&self, cb: impl Fn(String) + 'static) {
        *self.on_server_search.borrow_mut() = Some(Box::new(cb));
    }

    pub fn set_folders(&self, folders: &[MailFolder]) {
        *self.folders.borrow_mut() = folders.to_vec();
    }

    pub fn get_folders(&self) -> Vec<MailFolder> {
        self.folders.borrow().clone()
    }

    pub fn remove_message_by_uid(&self, uid: u32) {
        self.messages.borrow_mut().retain(|m| m.uid != uid);
        filter_and_sort(
            &self.model,
            &self.messages.borrow(),
            &self.filter_query.borrow(),
            self.sort_field.get(),
            self.sort_ascending.get(),
            &self.list_box,
            &self.sorted,
        );
    }

    pub fn update_flagged_status(&self, uid: u32, flagged: bool) {
        for msg in self.messages.borrow_mut().iter_mut() {
            if msg.uid == uid {
                msg.is_flagged = flagged;
            }
        }
        filter_and_sort(
            &self.model,
            &self.messages.borrow(),
            &self.filter_query.borrow(),
            self.sort_field.get(),
            self.sort_ascending.get(),
            &self.list_box,
            &self.sorted,
        );
    }

    pub fn update_read_status(&self, uid: u32, read: bool) {
        for msg in self.messages.borrow_mut().iter_mut() {
            if msg.uid == uid {
                msg.is_read = read;
            }
        }
        filter_and_sort(
            &self.model,
            &self.messages.borrow(),
            &self.filter_query.borrow(),
            self.sort_field.get(),
            self.sort_ascending.get(),
            &self.list_box,
            &self.sorted,
        );
    }

    fn create_message_row(msg: &MailMessageObject) -> gtk::Widget {
        let outer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        outer.set_margin_start(8);
        outer.set_margin_end(12);
        outer.set_margin_top(8);
        outer.set_margin_bottom(8);

        let avatar = adw::Avatar::new(32, Some(&msg.from_display()), true);
        outer.append(&avatar);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
        vbox.set_hexpand(true);

        let top_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        let from_label = gtk::Label::new(Some(&msg.from_display()));
        from_label.set_halign(gtk::Align::Start);
        from_label.set_hexpand(true);
        from_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        if !msg.is_read() {
            from_label.add_css_class("heading");
        }
        top_row.append(&from_label);

        if msg.is_flagged() {
            let star = gtk::Image::from_icon_name("starred-symbolic");
            star.set_pixel_size(14);
            top_row.append(&star);
        }

        if msg.has_attachments() {
            let clip = gtk::Image::from_icon_name("mail-attachment-symbolic");
            clip.set_pixel_size(14);
            clip.add_css_class("dim-label");
            top_row.append(&clip);
        }

        let date_label = gtk::Label::new(Some(&msg.date_display()));
        date_label.add_css_class("dim-label");
        date_label.add_css_class("caption");
        top_row.append(&date_label);

        vbox.append(&top_row);

        let subject_label = gtk::Label::new(Some(&msg.subject()));
        subject_label.set_halign(gtk::Align::Start);
        subject_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        if !msg.is_read() {
            subject_label.add_css_class("heading");
        }
        vbox.append(&subject_label);

        let preview = msg.preview();
        if !preview.is_empty() {
            let preview_label = gtk::Label::new(Some(&preview));
            preview_label.set_halign(gtk::Align::Start);
            preview_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            preview_label.add_css_class("dim-label");
            preview_label.add_css_class("caption");
            vbox.append(&preview_label);
        }

        outer.append(&vbox);
        outer.upcast()
    }
}

fn filter_and_sort(
    model: &gio::ListStore,
    messages: &[MailMessage],
    query: &str,
    field: SortField,
    ascending: bool,
    list_box: &gtk::ListBox,
    sorted_out: &std::cell::RefCell<Vec<MailMessage>>,
) {
    let query_lower = query.to_lowercase();
    let filtered: Vec<MailMessage> = if query_lower.is_empty() {
        messages.to_vec()
    } else {
        messages
            .iter()
            .filter(|m| {
                m.subject.to_lowercase().contains(&query_lower)
                    || m.from_display().to_lowercase().contains(&query_lower)
                    || m.body_text
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query_lower)
            })
            .cloned()
            .collect()
    };

    let mut sorted = filtered;
    sorted.sort_by(|a, b| {
        let cmp = match field {
            SortField::Date => a.date.cmp(&b.date),
            SortField::Sender => {
                let a_from = a.from_display().to_lowercase();
                let b_from = b.from_display().to_lowercase();
                a_from.cmp(&b_from)
            }
            SortField::Subject => {
                let a_subj = a.subject.to_lowercase();
                let b_subj = b.subject.to_lowercase();
                a_subj.cmp(&b_subj)
            }
            SortField::Unread => {
                a.is_read.cmp(&b.is_read) // false < true, unread first
            }
        };
        if ascending { cmp } else { cmp.reverse() }
    });

    model.remove_all();
    for msg in &sorted {
        model.append(&MailMessageObject::new(msg));
    }
    *sorted_out.borrow_mut() = sorted;

    if let Some(first) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first));
        list_box.grab_focus();
    }
}
