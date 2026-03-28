use crate::config::{AccountConfig, AppConfig};
use crate::i18n::{t, tf};
use crate::mail::disk_cache;
use crate::mail::error::MailError;
use crate::mail::imap_client::{self, ImapClient};
use crate::runtime;
use crate::state::AppState;
use crate::ui::account_dialog;
use crate::ui::compose_window;
use crate::ui::folder_list::FolderList;
use crate::ui::message_list::MessageList;
use crate::ui::message_view::MessageView;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub struct MailerWindow {
    pub window: adw::ApplicationWindow,
}

impl MailerWindow {
    pub fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(t("app.title"))
            .default_width(1200)
            .default_height(700)
            .icon_name("com.sayler.mailer")
            .build();

        let state = Rc::new(RefCell::new(AppState::new()));
        let imap_client: Rc<RefCell<Option<Arc<ImapClient>>>> = Rc::new(RefCell::new(None));

        let toast_overlay = adw::ToastOverlay::new();
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Header bar
        let header = adw::HeaderBar::new();

        let compose_btn = gtk::Button::from_icon_name("mail-message-new-symbolic");
        compose_btn.set_tooltip_text(Some(t("tooltip.new_message")));
        header.pack_start(&compose_btn);

        let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some(t("tooltip.refresh")));
        header.pack_start(&refresh_btn);

        let reply_btn = gtk::Button::from_icon_name("mail-reply-sender-symbolic");
        reply_btn.set_tooltip_text(Some(t("tooltip.reply")));
        header.pack_start(&reply_btn);

        let delete_btn = gtk::Button::from_icon_name("user-trash-symbolic");
        delete_btn.set_tooltip_text(Some(t("tooltip.delete")));
        header.pack_start(&delete_btn);

        let mark_read_btn = gtk::Button::from_icon_name("mail-read-symbolic");
        mark_read_btn.set_tooltip_text(Some(t("tooltip.mark_read")));
        header.pack_start(&mark_read_btn);

        let account_btn = gtk::Button::from_icon_name("avatar-default-symbolic");
        account_btn.set_tooltip_text(Some(t("tooltip.account_settings")));
        header.pack_end(&account_btn);

        let add_account_btn = gtk::Button::from_icon_name("list-add-symbolic");
        add_account_btn.set_tooltip_text(Some(t("account.add_title")));
        header.pack_end(&add_account_btn);

        // Account switcher
        let account_model = gtk::StringList::new(&[]);
        let account_dropdown = gtk::DropDown::new(Some(account_model.clone()), gtk::Expression::NONE);
        account_dropdown.set_tooltip_text(Some(t("account.switch")));
        {
            let config = AppConfig::load();
            for acc in &config.accounts {
                account_model.append(&acc.email);
            }
        }
        header.pack_end(&account_dropdown);

        main_box.append(&header);

        // Three-panel layout
        let outer_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        outer_paned.set_shrink_start_child(false);
        outer_paned.set_shrink_end_child(false);
        outer_paned.set_position(200);

        let inner_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        inner_paned.set_shrink_start_child(false);
        inner_paned.set_shrink_end_child(false);
        inner_paned.set_position(350);

        let folder_list = Rc::new(FolderList::new());
        let folder_scroll = gtk::ScrolledWindow::new();
        folder_scroll.set_child(Some(&folder_list.widget));
        folder_scroll.set_vexpand(true);
        folder_scroll.set_width_request(180);
        outer_paned.set_start_child(Some(&folder_scroll));

        let message_list = Rc::new(MessageList::new());
        message_list.widget.set_width_request(280);
        inner_paned.set_start_child(Some(&message_list.widget));

        let message_view = Rc::new(MessageView::new());
        message_view.widget.set_width_request(300);
        inner_paned.set_end_child(Some(&message_view.widget));

        outer_paned.set_end_child(Some(&inner_paned));
        main_box.append(&outer_paned);

        // Status bar
        let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        status_bar.set_margin_start(8);
        status_bar.set_margin_end(8);
        status_bar.set_margin_top(4);
        status_bar.set_margin_bottom(4);

        let status_label = Rc::new(gtk::Label::new(Some(t("status.not_connected"))));
        status_label.add_css_class("dim-label");
        status_label.add_css_class("caption");
        status_label.set_halign(gtk::Align::Start);
        status_bar.append(status_label.as_ref());

        let spinner = Rc::new(gtk::Spinner::new());
        status_bar.append(spinner.as_ref());

        main_box.append(&status_bar);

        toast_overlay.set_child(Some(&main_box));
        window.set_content(Some(&toast_overlay));

        // -- Signal handlers --

        // Folder click → fetch messages + start IDLE
        {
            let msg_list = message_list.clone();
            let msg_view = message_view.clone();
            let state2 = state.clone();
            let imap2 = imap_client.clone();
            let status2 = status_label.clone();
            let spinner2 = spinner.clone();
            let toast2 = toast_overlay.clone();

            folder_list.widget.connect_row_selected(move |_, row| {
                let Some(row) = row else { return };
                let folder_path = row.widget_name().to_string();

                state2.borrow_mut().selected_folder = Some(folder_path.clone());
                msg_view.clear();

                // Show cached messages immediately
                if let Some(cached) = disk_cache::load_messages(&folder_path) {
                    if !cached.is_empty() {
                        let count = cached.len();
                        state2.borrow_mut().set_messages(&folder_path, cached.clone(), false);
                        msg_list.set_messages(&cached);
                        status2.set_text(&tf("status.message_count", &[&count.to_string()]));
                    }
                }

                let Some(client) = imap2.borrow().clone() else {
                    return;
                };

                let folder_display = imap_client::decode_mutf7(&folder_path);
                status2.set_text(&tf("status.loading_folder", &[&folder_display]));
                spinner2.set_spinning(true);

                let msg_list = msg_list.clone();
                let state3 = state2.clone();
                let status3 = status2.clone();
                let spinner3 = spinner2.clone();
                let toast3 = toast2.clone();
                let client2 = client.clone();
                let folder_for_idle = folder_path.clone();

                runtime::spawn_on_main(
                    async move { client.fetch_messages(&folder_path, 50, 0).await },
                    move |result| {
                        spinner3.set_spinning(false);
                        match result {
                            Ok((messages, total)) => {
                                let count = messages.len();
                                let folder = state3
                                    .borrow()
                                    .selected_folder
                                    .clone()
                                    .unwrap_or_default();
                                {
                                    let mut st = state3.borrow_mut();
                                    st.folder_total = total;
                                    st.set_messages(&folder, messages.clone(), false);
                                }
                                disk_cache::save_messages(&folder, &messages);
                                msg_list.set_messages(&messages);
                                msg_list.set_has_more(state3.borrow().has_more());
                                status3.set_text(&tf("status.message_count", &[&count.to_string()]));

                                // Start IDLE on this folder
                                let status_idle = status3.clone();
                                let (idle_tx, idle_rx) = std::sync::mpsc::channel::<()>();
                                {
                                    let client_refresh = client2.clone();
                                    let msg_list_idle = msg_list.clone();
                                    let state_idle = state3.clone();
                                    let status_idle2 = status3.clone();
                                    let folder_idle = folder.clone();
                                    glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                                        if idle_rx.try_recv().is_ok() {
                                            // New mail arrived — send notification
                                            let app = msg_list_idle.widget.root()
                                                .and_then(|r| r.downcast::<adw::ApplicationWindow>().ok())
                                                .and_then(|w| w.application());
                                            if let Some(app) = app {
                                                let notification = gtk::gio::Notification::new(t("notification.new_mail"));
                                                notification.set_body(Some(t("notification.new_mail")));
                                                app.send_notification(Some("new-mail"), &notification);
                                            }
                                            // Refresh messages
                                            let client = client_refresh.clone();
                                            let msg_list = msg_list_idle.clone();
                                            let state = state_idle.clone();
                                            let status = status_idle2.clone();
                                            let folder = folder_idle.clone();
                                            runtime::spawn_on_main(
                                                async move { client.fetch_messages(&folder, 50, 0).await },
                                                move |result| {
                                                    if let Ok((messages, total)) = result {
                                                        let folder = state.borrow().selected_folder.clone().unwrap_or_default();
                                                        let mut st = state.borrow_mut();
                                                        st.folder_total = total;
                                                        st.set_messages(&folder, messages.clone(), false);
                                                        drop(st);
                                                        disk_cache::save_messages(&folder, &messages);
                                                        msg_list.set_messages(&messages);
                                                        msg_list.set_has_more(state.borrow().has_more());
                                                        let count = messages.len();
                                                        status.set_text(&tf("status.message_count", &[&count.to_string()]));
                                                    }
                                                },
                                            );
                                        }
                                        glib::ControlFlow::Continue
                                    });
                                }
                                runtime::spawn_on_main(
                                    async move {
                                        client2.start_idle(&folder_for_idle, move || {
                                            let _ = idle_tx.send(());
                                        }).await
                                    },
                                    move |result| {
                                        if result.is_ok() {
                                            status_idle.set_text(t("status.idle"));
                                        }
                                    },
                                );
                            }
                            Err(e) => {
                                status3.set_text(t("status.error_loading"));
                                toast3.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                            }
                        }
                    },
                );
            });
        }

        // Load more button
        {
            let msg_list_lm = message_list.clone();
            let imap_lm = imap_client.clone();
            let state_lm = state.clone();
            let status_lm = status_label.clone();
            let spinner_lm = spinner.clone();
            let toast_lm = toast_overlay.clone();

            message_list.load_more_btn.connect_clicked(move |_| {
                let Some(client) = imap_lm.borrow().clone() else { return };
                let folder = state_lm.borrow().selected_folder.clone().unwrap_or_default();
                let offset = state_lm.borrow().loaded_count;

                status_lm.set_text(t("status.refreshing"));
                spinner_lm.set_spinning(true);

                let msg_list = msg_list_lm.clone();
                let state2 = state_lm.clone();
                let status2 = status_lm.clone();
                let spinner2 = spinner_lm.clone();
                let toast2 = toast_lm.clone();

                runtime::spawn_on_main(
                    async move { client.fetch_messages(&folder, 50, offset).await },
                    move |result| {
                        spinner2.set_spinning(false);
                        match result {
                            Ok((messages, _total)) => {
                                let folder = state2.borrow().selected_folder.clone().unwrap_or_default();
                                state2.borrow_mut().set_messages(&folder, messages.clone(), true);
                                msg_list.append_messages(&messages);
                                msg_list.set_has_more(state2.borrow().has_more());
                                let count = state2.borrow().loaded_count;
                                status2.set_text(&tf("status.message_count", &[&count.to_string()]));
                            }
                            Err(e) => {
                                toast2.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                                status2.set_text(t("status.connected"));
                            }
                        }
                    },
                );
            });
        }

        // Server-side search
        {
            let msg_list_s = message_list.clone();
            let imap_s = imap_client.clone();
            let state_s = state.clone();
            let status_s = status_label.clone();
            let toast_s = toast_overlay.clone();

            message_list.set_on_server_search(move |query| {
                let Some(client) = imap_s.borrow().clone() else { return };
                let folder = state_s.borrow().selected_folder.clone().unwrap_or_default();
                let msg_list = msg_list_s.clone();
                let status = status_s.clone();
                let toast = toast_s.clone();

                status.set_text(t("status.refreshing"));
                runtime::spawn_on_main(
                    async move { client.search_messages(&folder, &query, 50).await },
                    move |result| match result {
                        Ok(messages) => {
                            let count = messages.len();
                            msg_list.set_messages(&messages);
                            msg_list.set_has_more(false);
                            status.set_text(&tf("search.results_count", &[&count.to_string()]));
                        }
                        Err(e) => {
                            toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                        }
                    },
                );
            });
        }

        // Message click → show preview
        {
            let msg_view2 = message_view.clone();
            let msg_list2 = message_list.clone();

            message_list.list_box.connect_row_selected(move |_, row| {
                let Some(row) = row else { return };
                let index = row.index() as usize;

                if let Some(msg) = msg_list2.get_sorted_message(index) {
                    msg_view2.show_message(&msg);
                }
            });
        }

        // Right-click context menu on message list
        {
            let gesture = gtk::GestureClick::new();
            gesture.set_button(3); // right click
            let msg_list_ctx = message_list.clone();
            let imap_ctx = imap_client.clone();
            let state_ctx = state.clone();
            let toast_ctx = toast_overlay.clone();
            let status_ctx = status_label.clone();
            let win_ctx = window.clone();

            gesture.connect_pressed(move |gesture, _n, x, y| {
                let lb = &msg_list_ctx.list_box;
                let Some(row) = lb.row_at_y(y as i32) else {
                    return;
                };
                lb.select_row(Some(&row));
                let i = row.index() as usize;
                let Some(msg) = msg_list_ctx.get_sorted_message(i) else {
                    return;
                };
                let uid = msg.uid;
                let is_read = msg.is_read;

                let menu = gtk::gio::Menu::new();
                menu.append(
                    Some(if is_read { t("menu.mark_unread") } else { t("menu.mark_read") }),
                    Some("ctx.toggle-read"),
                );
                menu.append(Some(t("menu.delete")), Some("ctx.delete"));
                menu.append(Some(t("menu.reply")), Some("ctx.reply"));

                // Move submenu
                let move_menu = gtk::gio::Menu::new();
                for folder in msg_list_ctx.get_folders() {
                    move_menu.append(
                        Some(&folder.name),
                        Some(&format!("ctx.move-to::{}", folder.path)),
                    );
                }
                menu.append_submenu(Some(t("menu.move_to")), &move_menu);

                let action_group = gtk::gio::SimpleActionGroup::new();

                // Toggle read action
                let toggle_action = gtk::gio::SimpleAction::new("toggle-read", None);
                {
                    let client = imap_ctx.borrow().clone();
                    let folder = state_ctx.borrow().selected_folder.clone().unwrap_or_default();
                    let ml = msg_list_ctx.clone();
                    let st = state_ctx.clone();
                    let toast = toast_ctx.clone();
                    toggle_action.connect_activate(move |_, _| {
                        let Some(client) = client.clone() else { return };
                        let new_read = !is_read;
                        let ml = ml.clone();
                        let st = st.clone();
                        let toast = toast.clone();
                        let folder = folder.clone();
                        runtime::spawn_on_main(
                            async move { client.mark_read(&folder, uid, new_read).await },
                            move |result| {
                                if result.is_ok() {
                                    st.borrow_mut().update_read_status(uid, new_read);
                                    ml.update_read_status(uid, new_read);
                                } else if let Err(e) = result {
                                    toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                                }
                            },
                        );
                    });
                }
                action_group.add_action(&toggle_action);

                // Delete action
                let del_action = gtk::gio::SimpleAction::new("delete", None);
                {
                    let client = imap_ctx.borrow().clone();
                    let folder = state_ctx.borrow().selected_folder.clone().unwrap_or_default();
                    let ml = msg_list_ctx.clone();
                    let st = state_ctx.clone();
                    let toast = toast_ctx.clone();
                    let status = status_ctx.clone();
                    del_action.connect_activate(move |_, _| {
                        let Some(client) = client.clone() else { return };
                        let ml = ml.clone();
                        let st = st.clone();
                        let toast = toast.clone();
                        let status = status.clone();
                        let folder = folder.clone();
                        status.set_text(t("status.deleting"));
                        runtime::spawn_on_main(
                            async move { client.delete_message(&folder, uid).await },
                            move |result| match result {
                                Ok(()) => {
                                    st.borrow_mut().remove_message(uid);
                                    ml.remove_message_by_uid(uid);
                                    toast.add_toast(adw::Toast::new(t("toast.deleted")));
                                    let count = ml.message_count();
                                    status.set_text(&tf("status.message_count", &[&count.to_string()]));
                                }
                                Err(e) => {
                                    toast.add_toast(adw::Toast::new(&tf("error.delete_failed", &[&e.to_string()])));
                                    status.set_text(t("status.connected"));
                                }
                            },
                        );
                    });
                }
                action_group.add_action(&del_action);

                // Reply action
                let reply_action = gtk::gio::SimpleAction::new("reply", None);
                {
                    let win = win_ctx.clone();
                    let msg2 = msg.clone();
                    let imap_reply = imap_ctx.clone();
                    reply_action.connect_activate(move |_, _| {
                        let config = AppConfig::load();
                        let Some(account) = config.first_account() else { return };
                        let reply_to = msg2.from.first().map(|a| a.email.clone()).unwrap_or_default();
                        let body = msg2.body_text.as_deref().unwrap_or("");
                        compose_window::show_reply_window(&win, account.clone(), &reply_to, &msg2.subject, body, imap_reply.borrow().clone());
                    });
                }
                action_group.add_action(&reply_action);

                // Move action
                let move_action = gtk::gio::SimpleAction::new("move-to", Some(&String::static_variant_type()));
                {
                    let client = imap_ctx.borrow().clone();
                    let folder = state_ctx.borrow().selected_folder.clone().unwrap_or_default();
                    let ml = msg_list_ctx.clone();
                    let st = state_ctx.clone();
                    let toast = toast_ctx.clone();
                    let status = status_ctx.clone();
                    move_action.connect_activate(move |_, param| {
                        let Some(dest) = param.and_then(|p| p.get::<String>()) else { return };
                        let Some(client) = client.clone() else { return };
                        let ml = ml.clone();
                        let st = st.clone();
                        let toast = toast.clone();
                        let status = status.clone();
                        let folder = folder.clone();
                        status.set_text(t("status.moving"));
                        runtime::spawn_on_main(
                            async move { client.move_message(&folder, uid, &dest).await },
                            move |result| match result {
                                Ok(()) => {
                                    st.borrow_mut().remove_message(uid);
                                    ml.remove_message_by_uid(uid);
                                    toast.add_toast(adw::Toast::new(t("toast.moved")));
                                    let count = ml.message_count();
                                    status.set_text(&tf("status.message_count", &[&count.to_string()]));
                                }
                                Err(e) => {
                                    toast.add_toast(adw::Toast::new(&tf("error.move_failed", &[&e.to_string()])));
                                    status.set_text(t("status.connected"));
                                }
                            },
                        );
                    });
                }
                action_group.add_action(&move_action);

                lb.insert_action_group("ctx", Some(&action_group));

                let popover = gtk::PopoverMenu::from_model(Some(&menu));
                popover.set_parent(&row);
                popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.set_has_arrow(false);
                popover.popup();

                gesture.set_state(gtk::EventSequenceState::Claimed);
            });

            message_list.list_box.add_controller(gesture);
        }

        // Compose button
        {
            let win2 = window.clone();
            let dd = account_dropdown.clone();
            let imap_compose = imap_client.clone();
            compose_btn.connect_clicked(move |_| {
                let config = AppConfig::load();
                let idx = dd.selected() as usize;
                if let Some(account) = config.accounts.get(idx) {
                    compose_window::show_compose_window(&win2, account.clone(), None, imap_compose.borrow().clone());
                }
            });
        }

        // Reply button
        {
            let win3 = window.clone();
            let msg_list3 = message_list.clone();
            let dd2 = account_dropdown.clone();
            let imap_reply = imap_client.clone();
            reply_btn.connect_clicked(move |_| {
                let config = AppConfig::load();
                let idx = dd2.selected() as usize;
                let Some(account) = config.accounts.get(idx) else {
                    return;
                };

                if let Some(row) = msg_list3.list_box.selected_row() {
                    let i = row.index() as usize;
                    if let Some(msg) = msg_list3.get_sorted_message(i) {
                        let reply_to = msg
                            .from
                            .first()
                            .map(|a| a.email.clone())
                            .unwrap_or_default();
                        let body = msg.body_text.as_deref().unwrap_or("");
                        compose_window::show_reply_window(
                            &win3,
                            account.clone(),
                            &reply_to,
                            &msg.subject,
                            body,
                            imap_reply.borrow().clone(),
                        );
                    }
                }
            });
        }

        // Delete button
        {
            let msg_list4 = message_list.clone();
            let imap5 = imap_client.clone();
            let state6 = state.clone();
            let toast5 = toast_overlay.clone();
            let status5 = status_label.clone();

            delete_btn.connect_clicked(move |_| {
                let Some(row) = msg_list4.list_box.selected_row() else {
                    return;
                };
                let i = row.index() as usize;
                let Some(msg) = msg_list4.get_sorted_message(i) else {
                    return;
                };
                let uid = msg.uid;

                let Some(client) = imap5.borrow().clone() else {
                    return;
                };
                let folder = state6
                    .borrow()
                    .selected_folder
                    .clone()
                    .unwrap_or_default();

                status5.set_text(t("status.deleting"));
                let msg_list = msg_list4.clone();
                let state7 = state6.clone();
                let toast = toast5.clone();
                let status = status5.clone();

                runtime::spawn_on_main(
                    async move { client.delete_message(&folder, uid).await },
                    move |result| match result {
                        Ok(()) => {
                            state7.borrow_mut().remove_message(uid);
                            msg_list.remove_message_by_uid(uid);
                            toast.add_toast(adw::Toast::new(t("toast.deleted")));
                            let count = msg_list.message_count();
                            status.set_text(&tf("status.message_count", &[&count.to_string()]));
                        }
                        Err(e) => {
                            toast.add_toast(adw::Toast::new(&tf("error.delete_failed", &[&e.to_string()])));
                            status.set_text(t("status.connected"));
                        }
                    },
                );
            });
        }

        // Mark read/unread toggle button
        {
            let msg_list5 = message_list.clone();
            let imap6 = imap_client.clone();
            let state8 = state.clone();
            let toast6 = toast_overlay.clone();

            mark_read_btn.connect_clicked(move |_| {
                let Some(row) = msg_list5.list_box.selected_row() else {
                    return;
                };
                let i = row.index() as usize;
                let Some(msg) = msg_list5.get_sorted_message(i) else {
                    return;
                };
                let uid = msg.uid;
                let new_read = !msg.is_read;

                let Some(client) = imap6.borrow().clone() else {
                    return;
                };
                let folder = state8
                    .borrow()
                    .selected_folder
                    .clone()
                    .unwrap_or_default();

                let msg_list = msg_list5.clone();
                let state9 = state8.clone();
                let toast = toast6.clone();

                runtime::spawn_on_main(
                    async move { client.mark_read(&folder, uid, new_read).await },
                    move |result| match result {
                        Ok(()) => {
                            state9.borrow_mut().update_read_status(uid, new_read);
                            msg_list.update_read_status(uid, new_read);
                        }
                        Err(e) => {
                            toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                        }
                    },
                );
            });
        }

        // Account button
        {
            let win4 = window.clone();
            let folder_list2 = folder_list.clone();
            let imap3 = imap_client.clone();
            let status3 = status_label.clone();
            let spinner3 = spinner.clone();
            let toast3 = toast_overlay.clone();
            let msg_list_for_folders = message_list.clone();
            let dd_for_edit = account_dropdown.clone();

            account_btn.connect_clicked(move |_| {
                let account_dropdown = dd_for_edit.clone();
                let folder_list = folder_list2.clone();
                let imap = imap3.clone();
                let status = status3.clone();
                let spinner = spinner3.clone();
                let toast = toast3.clone();
                let ml = msg_list_for_folders.clone();

                // Get current account for editing
                let app_config = AppConfig::load();
                let dd_selected = account_dropdown.selected() as usize;
                let existing = app_config
                    .accounts
                    .get(dd_selected)
                    .map(|a| (dd_selected, a.clone()));

                let is_editing = existing.is_some();
                account_dialog::show_account_dialog(&win4, existing, move |config| {
                    // When editing without changing password, don't reconnect
                    if is_editing && config.password.is_empty() {
                        return;
                    }
                    connect_account(
                        config,
                        imap.clone(),
                        folder_list.clone(),
                        status.clone(),
                        spinner.clone(),
                        toast.clone(),
                        ml.clone(),
                    );
                });
            });
        }

        // Add account button
        {
            let win5 = window.clone();
            let folder_list5 = folder_list.clone();
            let imap5 = imap_client.clone();
            let status5 = status_label.clone();
            let spinner5 = spinner.clone();
            let toast5 = toast_overlay.clone();
            let ml5 = message_list.clone();
            let am5 = account_model.clone();
            let dd5 = account_dropdown.clone();

            add_account_btn.connect_clicked(move |_| {
                let folder_list = folder_list5.clone();
                let imap = imap5.clone();
                let status = status5.clone();
                let spinner = spinner5.clone();
                let toast = toast5.clone();
                let ml = ml5.clone();
                let am = am5.clone();
                let dd = dd5.clone();

                account_dialog::show_account_dialog(&win5, None, move |config| {
                    am.append(&config.email);
                    let new_idx = am.n_items() - 1;
                    dd.set_selected(new_idx);
                    connect_account(
                        config,
                        imap.clone(),
                        folder_list.clone(),
                        status.clone(),
                        spinner.clone(),
                        toast.clone(),
                        ml.clone(),
                    );
                });
            });
        }

        // Refresh button
        {
            let folder_list3 = folder_list.clone();
            let imap4 = imap_client.clone();
            let status4 = status_label.clone();
            let spinner4 = spinner.clone();
            let toast4 = toast_overlay.clone();
            let msg_list_for_refresh = message_list.clone();

            refresh_btn.connect_clicked(move |_| {
                let Some(client) = imap4.borrow().clone() else {
                    toast4.add_toast(adw::Toast::new(t("toast.no_account")));
                    return;
                };

                status4.set_text(t("status.refreshing"));
                spinner4.set_spinning(true);

                let folder_list = folder_list3.clone();
                let status = status4.clone();
                let spinner = spinner4.clone();
                let toast = toast4.clone();
                let ml = msg_list_for_refresh.clone();
                let client2 = client.clone();

                runtime::spawn_on_main(
                    async move { client.list_folders().await },
                    move |result| {
                        spinner.set_spinning(false);
                        match result {
                            Ok(folders) => {
                                ml.set_folders(&folders);
                                folder_list.populate(&folders);
                                status.set_text(t("status.connected"));

                                // Fetch unread counts
                                let folder_paths: Vec<String> =
                                    folders.iter().map(|f| f.path.clone()).collect();
                                let fl2 = folder_list.clone();
                                runtime::spawn_on_main(
                                    async move {
                                        client2.fetch_unread_counts(&folder_paths).await
                                    },
                                    move |result| {
                                        if let Ok(counts) = result {
                                            fl2.update_unread_counts(&counts);
                                        }
                                    },
                                );
                            }
                            Err(e) => {
                                status.set_text(t("status.refresh_failed"));
                                toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                            }
                        }
                    },
                );
            });
        }

        // Keyboard shortcuts
        let app2 = app.clone();
        app2.set_accels_for_action("win.compose", &["<Ctrl>n"]);
        app2.set_accels_for_action("win.reply", &["<Ctrl>r"]);
        app2.set_accels_for_action("win.delete", &["Delete"]);
        app2.set_accels_for_action("win.search-focus", &["<Ctrl>f"]);
        app2.set_accels_for_action("win.refresh", &["F5"]);

        // Register actions
        let action_group = gtk::gio::SimpleActionGroup::new();

        let compose_action = gtk::gio::SimpleAction::new("compose", None);
        let compose_btn2 = compose_btn.clone();
        compose_action.connect_activate(move |_, _| compose_btn2.emit_clicked());
        action_group.add_action(&compose_action);

        let reply_action = gtk::gio::SimpleAction::new("reply", None);
        let reply_btn2 = reply_btn.clone();
        reply_action.connect_activate(move |_, _| reply_btn2.emit_clicked());
        action_group.add_action(&reply_action);

        let delete_action = gtk::gio::SimpleAction::new("delete", None);
        let delete_btn2 = delete_btn.clone();
        delete_action.connect_activate(move |_, _| delete_btn2.emit_clicked());
        action_group.add_action(&delete_action);

        let search_action = gtk::gio::SimpleAction::new("search-focus", None);
        let search_entry = message_list.search_entry.clone();
        search_action.connect_activate(move |_, _| {
            search_entry.grab_focus();
        });
        action_group.add_action(&search_action);

        let refresh_action = gtk::gio::SimpleAction::new("refresh", None);
        let refresh_btn2 = refresh_btn.clone();
        refresh_action.connect_activate(move |_, _| refresh_btn2.emit_clicked());
        action_group.add_action(&refresh_action);

        window.insert_action_group("win", Some(&action_group));

        // Account switcher handler
        {
            let imap7 = imap_client.clone();
            let fl = folder_list.clone();
            let sl = status_label.clone();
            let sp = spinner.clone();
            let to = toast_overlay.clone();
            let ml = message_list.clone();
            let mv = message_view.clone();

            account_dropdown.connect_selected_notify(move |dd| {
                let idx = dd.selected() as usize;
                let config = AppConfig::load();
                if let Some(account) = config.accounts.get(idx) {
                    // Stop IDLE on current client
                    if let Some(client) = imap7.borrow().as_ref() {
                        let client = client.clone();
                        runtime::spawn_on_main(
                            async move { client.stop_idle().await; Ok::<(), MailError>(()) },
                            |_| {},
                        );
                    }
                    mv.clear();
                    connect_account(
                        account.clone(),
                        imap7.clone(),
                        fl.clone(),
                        sl.clone(),
                        sp.clone(),
                        to.clone(),
                        ml.clone(),
                    );
                }
            });
        }

        // Folder management: create
        {
            let imap_fm = imap_client.clone();
            let fl_fm = folder_list.clone();
            let status_fm = status_label.clone();
            let spinner_fm = spinner.clone();
            let toast_fm = toast_overlay.clone();
            let ml_fm = message_list.clone();
            let win_fm = window.clone();
            *folder_list.on_create_folder.borrow_mut() = Some(Box::new(move || {
                let dialog = adw::Window::builder()
                    .title(t("folder.create"))
                    .default_width(400)
                    .default_height(150)
                    .transient_for(&win_fm)
                    .modal(true)
                    .build();
                let dbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let dheader = adw::HeaderBar::new();
                let ok_btn = gtk::Button::with_label("OK");
                ok_btn.add_css_class("suggested-action");
                dheader.pack_end(&ok_btn);
                dbox.append(&dheader);
                let entry = gtk::Entry::new();
                entry.set_placeholder_text(Some(t("folder.name_prompt")));
                entry.set_margin_start(16);
                entry.set_margin_end(16);
                entry.set_margin_top(16);
                entry.set_margin_bottom(16);
                dbox.append(&entry);
                dialog.set_content(Some(&dbox));

                let imap = imap_fm.clone();
                let fl = fl_fm.clone();
                let status = status_fm.clone();
                let spinner = spinner_fm.clone();
                let toast = toast_fm.clone();
                let ml = ml_fm.clone();
                let d = dialog.clone();
                ok_btn.connect_clicked(move |_| {
                    let name = entry.text().to_string();
                    if name.is_empty() { return; }
                    let Some(client) = imap.borrow().clone() else { return };
                    let fl = fl.clone();
                    let status = status.clone();
                    let spinner = spinner.clone();
                    let toast = toast.clone();
                    let ml = ml.clone();
                    let client2 = client.clone();
                    let name2 = name.clone();
                    d.close();
                    spinner.set_spinning(true);
                    runtime::spawn_on_main(
                        async move { client.create_folder(&name).await },
                        move |result| {
                            spinner.set_spinning(false);
                            match result {
                                Ok(()) => {
                                    toast.add_toast(adw::Toast::new(&tf("toast.folder_created", &[&name2])));
                                    refresh_folders(client2, fl, status, ml);
                                }
                                Err(e) => toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()]))),
                            }
                        },
                    );
                });
                dialog.present();
            }));
        }

        // Folder management: rename
        {
            let imap_fm = imap_client.clone();
            let fl_fm = folder_list.clone();
            let status_fm = status_label.clone();
            let spinner_fm = spinner.clone();
            let toast_fm = toast_overlay.clone();
            let ml_fm = message_list.clone();
            let win_fm = window.clone();
            *folder_list.on_rename_folder.borrow_mut() = Some(Box::new(move |old_name: String| {
                let dialog = adw::Window::builder()
                    .title(t("folder.rename"))
                    .default_width(400)
                    .default_height(150)
                    .transient_for(&win_fm)
                    .modal(true)
                    .build();
                let dbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let dheader = adw::HeaderBar::new();
                let ok_btn = gtk::Button::with_label("OK");
                ok_btn.add_css_class("suggested-action");
                dheader.pack_end(&ok_btn);
                dbox.append(&dheader);
                let entry = gtk::Entry::new();
                entry.set_text(&old_name);
                entry.set_margin_start(16);
                entry.set_margin_end(16);
                entry.set_margin_top(16);
                entry.set_margin_bottom(16);
                dbox.append(&entry);
                dialog.set_content(Some(&dbox));

                let imap = imap_fm.clone();
                let fl = fl_fm.clone();
                let status = status_fm.clone();
                let spinner = spinner_fm.clone();
                let toast = toast_fm.clone();
                let ml = ml_fm.clone();
                let d = dialog.clone();
                ok_btn.connect_clicked(move |_| {
                    let new_name = entry.text().to_string();
                    if new_name.is_empty() || new_name == old_name { return; }
                    let Some(client) = imap.borrow().clone() else { return };
                    let fl = fl.clone();
                    let status = status.clone();
                    let spinner = spinner.clone();
                    let toast = toast.clone();
                    let ml = ml.clone();
                    let client2 = client.clone();
                    let old = old_name.clone();
                    let new2 = new_name.clone();
                    d.close();
                    spinner.set_spinning(true);
                    runtime::spawn_on_main(
                        async move { client.rename_folder(&old, &new_name).await },
                        move |result| {
                            spinner.set_spinning(false);
                            match result {
                                Ok(()) => {
                                    toast.add_toast(adw::Toast::new(&tf("toast.folder_renamed", &[&new2])));
                                    refresh_folders(client2, fl, status, ml);
                                }
                                Err(e) => toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()]))),
                            }
                        },
                    );
                });
                dialog.present();
            }));
        }

        // Folder management: delete
        {
            let imap_fm = imap_client.clone();
            let fl_fm = folder_list.clone();
            let status_fm = status_label.clone();
            let spinner_fm = spinner.clone();
            let toast_fm = toast_overlay.clone();
            let ml_fm = message_list.clone();
            *folder_list.on_delete_folder.borrow_mut() = Some(Box::new(move |name: String| {
                let Some(client) = imap_fm.borrow().clone() else { return };
                let fl = fl_fm.clone();
                let status = status_fm.clone();
                let spinner = spinner_fm.clone();
                let toast = toast_fm.clone();
                let ml = ml_fm.clone();
                let client2 = client.clone();
                let name2 = name.clone();
                spinner.set_spinning(true);
                runtime::spawn_on_main(
                    async move { client.delete_folder(&name).await },
                    move |result| {
                        spinner.set_spinning(false);
                        match result {
                            Ok(()) => {
                                toast.add_toast(adw::Toast::new(&tf("toast.folder_deleted", &[&name2])));
                                refresh_folders(client2, fl, status, ml);
                            }
                            Err(e) => toast.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()]))),
                        }
                    },
                );
            }));
        }

        // Drafts button (popover near compose)
        {
            let drafts_btn = gtk::MenuButton::new();
            drafts_btn.set_icon_name("document-edit-symbolic");
            drafts_btn.set_tooltip_text(Some(t("draft.open")));

            let popover = gtk::Popover::new();

            let win_d = window.clone();
            let dd_d = account_dropdown.clone();
            let popover_d = popover.clone();
            let imap_drafts = imap_client.clone();
            popover.connect_show(move |pop| {
                // Rebuild drafts list each time
                let drafts_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
                drafts_box.set_margin_start(8);
                drafts_box.set_margin_end(8);
                drafts_box.set_margin_top(8);
                drafts_box.set_margin_bottom(8);

                let drafts = crate::mail::drafts::list_drafts();
                if drafts.is_empty() {
                    let lbl = gtk::Label::new(Some(t("draft.empty")));
                    lbl.add_css_class("dim-label");
                    drafts_box.append(&lbl);
                } else {
                    for draft in drafts {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                        let label_text = if draft.subject.is_empty() {
                            draft.to.clone()
                        } else {
                            draft.subject.clone()
                        };
                        let btn = gtk::Button::with_label(&label_text);
                        btn.add_css_class("flat");
                        btn.set_hexpand(true);
                        let win = win_d.clone();
                        let dd = dd_d.clone();
                        let pop2 = popover_d.clone();
                        let d = draft.clone();
                        let imap_d = imap_drafts.clone();
                        btn.connect_clicked(move |_| {
                            let config = AppConfig::load();
                            let idx = dd.selected() as usize;
                            if let Some(account) = config.accounts.get(idx) {
                                compose_window::show_compose_window(&win, account.clone(), Some(d.clone()), imap_d.borrow().clone());
                            }
                            pop2.popdown();
                        });
                        row.append(&btn);

                        let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
                        del_btn.add_css_class("flat");
                        let draft_id = draft.id.clone();
                        let row_ref = row.clone();
                        let db = drafts_box.clone();
                        del_btn.connect_clicked(move |_| {
                            crate::mail::drafts::delete_draft(&draft_id).ok();
                            db.remove(&row_ref);
                        });
                        row.append(&del_btn);

                        drafts_box.append(&row);
                    }
                }
                pop.set_child(Some(&drafts_box));
            });

            drafts_btn.set_popover(Some(&popover));
            header.pack_start(&drafts_btn);
        }

        // Auto-connect if account exists
        let config = AppConfig::load();
        if let Some(account) = config.first_account() {
            connect_account(
                account.clone(),
                imap_client.clone(),
                folder_list.clone(),
                status_label.clone(),
                spinner.clone(),
                toast_overlay.clone(),
                message_list.clone(),
            );
        }

        Self { window }
    }
}

fn refresh_folders(
    client: Arc<ImapClient>,
    folder_list: Rc<FolderList>,
    status_label: Rc<gtk::Label>,
    message_list: Rc<MessageList>,
) {
    let client2 = client.clone();
    runtime::spawn_on_main(
        async move { client.list_folders().await },
        move |result| {
            if let Ok(folders) = result {
                disk_cache::save_folders(&folders);
                message_list.set_folders(&folders);
                folder_list.populate(&folders);
                status_label.set_text(t("status.connected"));

                let folder_paths: Vec<String> = folders.iter().map(|f| f.path.clone()).collect();
                let fl2 = folder_list.clone();
                runtime::spawn_on_main(
                    async move { client2.fetch_unread_counts(&folder_paths).await },
                    move |result| {
                        if let Ok(counts) = result {
                            fl2.update_unread_counts(&counts);
                        }
                    },
                );
            }
        },
    );
}

fn connect_account(
    config: AccountConfig,
    imap_client: Rc<RefCell<Option<Arc<ImapClient>>>>,
    folder_list: Rc<FolderList>,
    status_label: Rc<gtk::Label>,
    spinner: Rc<gtk::Spinner>,
    toast_overlay: adw::ToastOverlay,
    message_list: Rc<MessageList>,
) {
    // If password is empty, try loading from keyring first
    if config.password.is_empty() {
        let email = config.email.clone();
        status_label.set_text(t("status.loading_password"));
        spinner.set_spinning(true);

        let config = config.clone();
        let imap_client = imap_client.clone();
        let folder_list = folder_list.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let toast_overlay = toast_overlay.clone();
        let message_list = message_list.clone();

        runtime::spawn_on_main(
            async move { crate::config::load_password(&email).await },
            move |result| {
                let mut config = config;
                match result {
                    Ok(password) => {
                        config.password = password;
                    }
                    Err(e) => {
                        log::warn!("Keyring load failed: {e}");
                        spinner.set_spinning(false);
                        status_label.set_text(t("status.connection_failed"));
                        toast_overlay.add_toast(adw::Toast::new(&tf("error.keyring_failed", &[&e])));
                        return;
                    }
                }
                connect_account_inner(
                    config,
                    imap_client,
                    folder_list,
                    status_label,
                    spinner,
                    toast_overlay,
                    message_list,
                );
            },
        );
        return;
    }

    connect_account_inner(config, imap_client, folder_list, status_label, spinner, toast_overlay, message_list);
}

fn connect_account_inner(
    config: AccountConfig,
    imap_client: Rc<RefCell<Option<Arc<ImapClient>>>>,
    folder_list: Rc<FolderList>,
    status_label: Rc<gtk::Label>,
    spinner: Rc<gtk::Spinner>,
    toast_overlay: adw::ToastOverlay,
    message_list: Rc<MessageList>,
) {
    // Show cached folders immediately while connecting
    if let Some(cached_folders) = disk_cache::load_folders() {
        if !cached_folders.is_empty() {
            message_list.set_folders(&cached_folders);
            folder_list.populate(&cached_folders);
        }
    }

    let client = Arc::new(ImapClient::new(config));
    *imap_client.borrow_mut() = Some(client.clone());

    status_label.set_text(t("status.connecting"));
    spinner.set_spinning(true);

    let client2 = client.clone();
    runtime::spawn_on_main(
        async move { client.list_folders().await },
        move |result| {
            spinner.set_spinning(false);
            match result {
                Ok(folders) => {
                    disk_cache::save_folders(&folders);
                    status_label.set_text(t("status.connected"));
                    message_list.set_folders(&folders);
                    folder_list.populate(&folders);

                    // Fetch unread counts
                    let folder_paths: Vec<String> =
                        folders.iter().map(|f| f.path.clone()).collect();
                    let fl2 = folder_list.clone();
                    runtime::spawn_on_main(
                        async move { client2.fetch_unread_counts(&folder_paths).await },
                        move |result| {
                            if let Ok(counts) = result {
                                fl2.update_unread_counts(&counts);
                            }
                        },
                    );
                }
                Err(e) => {
                    status_label.set_text(t("status.connection_failed"));
                    toast_overlay.add_toast(adw::Toast::new(&tf("error.generic", &[&e.to_string()])));
                }
            }
        },
    );
}
