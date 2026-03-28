use gtk::glib;
use once_cell::sync::Lazy;
use std::future::Future;
use tokio::runtime::Runtime;

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("Failed to create Tokio runtime")
});

/// Spawn an async task on the Tokio runtime and deliver the result
/// back to the GTK main loop via a callback.
pub fn spawn_on_main<F, T>(future: F, callback: impl FnOnce(T) + 'static)
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    RUNTIME.spawn(async move {
        let _ = tx.send(future.await);
    });
    glib::spawn_future_local(async move {
        if let Ok(result) = rx.await {
            callback(result);
        }
    });
}
