use crate::i18n::t;
use crate::mail::models::MailFolder;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct FolderList {
    pub widget: gtk::ListBox,
    pub on_create_folder: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    pub on_rename_folder: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
    pub on_delete_folder: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
}

impl FolderList {
    pub fn new() -> Self {
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.add_css_class("navigation-sidebar");

        let on_create_folder: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let on_rename_folder: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));
        let on_delete_folder: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));

        // Right-click context menu
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        let ocf = on_create_folder.clone();
        let orf = on_rename_folder.clone();
        let odf = on_delete_folder.clone();
        let lb = list_box.clone();

        gesture.connect_pressed(move |gesture, _n, x, y| {
            let folder_path = lb.row_at_y(y as i32).map(|r| r.widget_name().to_string());

            let menu = gtk::gio::Menu::new();
            menu.append(Some(t("folder.create")), Some("fctx.create"));

            if let Some(ref _path) = folder_path {
                menu.append(Some(t("folder.rename")), Some("fctx.rename"));
                menu.append(Some(t("folder.delete")), Some("fctx.delete"));
            }

            let action_group = gtk::gio::SimpleActionGroup::new();

            let create_action = gtk::gio::SimpleAction::new("create", None);
            let ocf2 = ocf.clone();
            create_action.connect_activate(move |_, _| {
                if let Some(ref cb) = *ocf2.borrow() {
                    cb();
                }
            });
            action_group.add_action(&create_action);

            if let Some(ref path) = folder_path {
                let rename_action = gtk::gio::SimpleAction::new("rename", None);
                let orf2 = orf.clone();
                let path_r = path.clone();
                rename_action.connect_activate(move |_, _| {
                    if let Some(ref cb) = *orf2.borrow() {
                        cb(path_r.clone());
                    }
                });
                action_group.add_action(&rename_action);

                let delete_action = gtk::gio::SimpleAction::new("delete", None);
                let odf2 = odf.clone();
                let path_d = path.clone();
                delete_action.connect_activate(move |_, _| {
                    if let Some(ref cb) = *odf2.borrow() {
                        cb(path_d.clone());
                    }
                });
                action_group.add_action(&delete_action);
            }

            lb.insert_action_group("fctx", Some(&action_group));

            let row_widget: gtk::Widget = if let Some(row) = lb.row_at_y(y as i32) {
                row.into()
            } else {
                lb.clone().into()
            };
            let popover = gtk::PopoverMenu::from_model(Some(&menu));
            popover.set_parent(&row_widget);
            popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover.set_has_arrow(false);
            popover.popup();

            gesture.set_state(gtk::EventSequenceState::Claimed);
        });

        list_box.add_controller(gesture);

        Self {
            widget: list_box,
            on_create_folder,
            on_rename_folder,
            on_delete_folder,
        }
    }

    pub fn populate(&self, folders: &[MailFolder]) {
        // Remove all children
        while let Some(child) = self.widget.first_child() {
            self.widget.remove(&child);
        }

        for folder in folders {
            let row = self.create_folder_row(folder);
            self.widget.append(&row);
        }

        // Select first row
        if let Some(first) = self.widget.row_at_index(0) {
            self.widget.select_row(Some(&first));
        }
    }

    fn create_folder_row(&self, folder: &MailFolder) -> gtk::ListBoxRow {
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);

        let icon_name = match folder.path.to_uppercase().as_str() {
            "INBOX" => "mail-inbox-symbolic",
            path if path.contains("SENT") => "mail-send-symbolic",
            path if path.contains("DRAFT") => "document-edit-symbolic",
            path if path.contains("TRASH") || path.contains("DELETED") => "user-trash-symbolic",
            path if path.contains("SPAM") || path.contains("JUNK") => "mail-mark-junk-symbolic",
            path if path.contains("STARRED") || path.contains("FLAGGED") => {
                "starred-symbolic"
            }
            path if path.contains("ARCHIVE") => "folder-symbolic",
            _ => "folder-symbolic",
        };

        let icon = gtk::Image::from_icon_name(icon_name);
        hbox.append(&icon);

        let label = gtk::Label::new(Some(&folder.name));
        label.set_halign(gtk::Align::Start);
        label.set_hexpand(true);
        hbox.append(&label);

        if folder.unread_count > 0 {
            let badge = gtk::Label::new(Some(&folder.unread_count.to_string()));
            badge.add_css_class("dim-label");
            hbox.append(&badge);
        }

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&hbox));

        // Store folder path as widget name for retrieval on click
        row.set_widget_name(&folder.path);

        row
    }

    pub fn update_unread_counts(&self, counts: &[(String, u32)]) {
        let mut i = 0;
        while let Some(row) = self.widget.row_at_index(i) {
            let folder_path = row.widget_name().to_string();
            if let Some((_, count)) = counts.iter().find(|(p, _)| p == &folder_path) {
                // Find the badge label or add one
                if let Some(hbox) = row.child() {
                    let hbox = hbox.downcast_ref::<gtk::Box>().unwrap();
                    // Remove old badge if exists (last child if it's a label with dim-label)
                    let mut last = hbox.last_child();
                    if let Some(ref widget) = last {
                        if widget.css_classes().iter().any(|c| c == "badge-label") {
                            hbox.remove(widget);
                            last = hbox.last_child();
                        }
                    }
                    let _ = last;
                    if *count > 0 {
                        let badge = gtk::Label::new(Some(&count.to_string()));
                        badge.add_css_class("dim-label");
                        badge.add_css_class("badge-label");
                        hbox.append(&badge);
                    }
                }
            }
            i += 1;
        }
    }
}
