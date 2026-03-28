use crate::ui::window::MailerWindow;
use adw::prelude::*;

pub fn build_app() -> adw::Application {
    let app = adw::Application::builder()
        .application_id("com.sayler.mailer")
        .build();

    app.connect_activate(|app| {
        let window = MailerWindow::new(app);
        window.window.present();
    });

    app
}
