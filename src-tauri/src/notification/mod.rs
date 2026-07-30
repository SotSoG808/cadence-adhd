//! Notification dispatcher.
//!
//! Fires Windows toast notifications via tauri-plugin-notification and
//! optionally mirrors them to a phone/Garmin watch via ntfy.sh.

pub mod ntfy;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone)]
pub struct Reminder {
    pub title: String,
    pub body: String,
    pub task_id: Option<String>,
}

/// Send a desktop toast (and optionally ntfy push if configured).
pub async fn dispatch(app: &AppHandle, reminder: &Reminder, ntfy_topic: &str, ntfy_enabled: bool) {
    // Windows toast
    let _ = app
        .notification()
        .builder()
        .title(&reminder.title)
        .body(&reminder.body)
        .show();

    // ntfy mirror
    if ntfy_enabled && !ntfy_topic.is_empty() {
        let _ = ntfy::push(ntfy_topic, &reminder.title, &reminder.body).await;
    }
}
