mod app;
mod config;
pub mod i18n;
mod mail;
mod runtime;
mod state;
mod ui;

use gtk::prelude::*;

fn main() {
    env_logger::init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let application = app::build_app();
    application.connect_startup(|_| {
        gtk::Window::set_default_icon_name("com.sayler.mailer");
    });
    application.run();
}
