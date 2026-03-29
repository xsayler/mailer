mod app;
mod config;
pub mod i18n;
mod mail;
mod runtime;
mod state;
mod ui;

use gtk::prelude::*;

fn main() {
    // Setup logging: terminal + file
    let log_path = config::AppConfig::config_dir().join("mailer.log");
    // Truncate log if > 1MB
    if let Ok(meta) = std::fs::metadata(&log_path) {
        if meta.len() > 1_048_576 {
            std::fs::write(&log_path, "").ok();
        }
    }
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);

    if let Ok(file) = log_file {
        use std::io::Write;
        let file = std::sync::Mutex::new(file);
        env_logger::Builder::from_default_env()
            .format(move |buf, record| {
                let line = format!(
                    "[{} {} {}] {}\n",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.target(),
                    record.args()
                );
                // Write to both stderr (default) and file
                if let Ok(mut f) = file.lock() {
                    let _ = f.write_all(line.as_bytes());
                }
                write!(buf, "{line}")
            })
            .init();
    } else {
        env_logger::init();
    }

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let application = app::build_app();
    application.connect_startup(|_| {
        gtk::Window::set_default_icon_name("com.sayler.mailer");
    });
    application.run();
}
