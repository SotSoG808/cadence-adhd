//! Reminder engine.
//!
//! Runs on a dedicated Tokio background task that survives window close.
//! Every 60 seconds it:
//!   1. Loads all pending tasks from the store.
//!   2. Evaluates eligibility using the domain rules (focus mode, snooze,
//!      deferral, sequence, scheduled days, quiet hours).
//!   3. For each task whose scheduled_at has passed and hasn't been notified
//!      in the last reminder_cooldown_secs, fires a notification.

use crate::{
    domain::{FocusMode, Task},
    notification::{dispatch, Reminder},
    store::Store,
};
use anyhow::Result;
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::AppHandle;

pub const TICK_SECS: u64 = 60;
pub const COOLDOWN_SECS: i64 = 300; // 5 min between repeat nags

pub struct EngineState {
    /// Last time (UTC epoch secs) each task was reminded
    pub last_reminded: HashMap<String, i64>,
}

pub async fn run_loop(
    app: AppHandle,
    store: Arc<Mutex<Store>>,
    state: Arc<Mutex<EngineState>>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(TICK_SECS));
    loop {
        interval.tick().await;
        if let Err(e) = tick(&app, &store, &state).await {
            eprintln!("[engine] tick error: {e}");
        }
    }
}

async fn tick(
    app: &AppHandle,
    store: &Arc<Mutex<Store>>,
    state: &Arc<Mutex<EngineState>>,
) -> Result<()> {
    let (tasks, focus_mode, ntfy_topic, ntfy_enabled, completed_ids) = {
        let s = store.lock().unwrap();
        let tasks = load_pending_tasks(&s)?;
        let focus_mode = load_focus_mode(&s)?;
        let ntfy_topic = load_setting(&s, "ntfy_topic")?;
        let ntfy_enabled = load_setting(&s, "ntfy_enabled")? == "1";
        let completed_ids = load_todays_completed_ids(&s)?;
        (tasks, focus_mode, ntfy_topic, ntfy_enabled, completed_ids)
    };

    let now = Utc::now();
    let now_epoch = now.timestamp();

    for task in &tasks {
        if !task.eligible(now, focus_mode, &completed_ids) {
            continue;
        }
        let Some(sched) = task.scheduled_at else { continue };
        if sched > now {
            continue;
        }

        let last = state
            .lock()
            .unwrap()
            .last_reminded
            .get(&task.id)
            .copied()
            .unwrap_or(0);
        if now_epoch - last < COOLDOWN_SECS {
            continue;
        }

        let title = decrypt_or_fallback(store, &task.id);
        let reminder = Reminder {
            title: "Cadence reminder".into(),
            body: title,
            task_id: Some(task.id.clone()),
        };
        dispatch(app, &reminder, &ntfy_topic, ntfy_enabled).await;
        state
            .lock()
            .unwrap()
            .last_reminded
            .insert(task.id.clone(), now_epoch);
    }
    Ok(())
}

fn decrypt_or_fallback(store: &Arc<Mutex<Store>>, task_id: &str) -> String {
    let s = store.lock().unwrap();
    s.conn
        .query_row(
            "SELECT title_enc FROM tasks WHERE id = ?1",
            [task_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|enc| s.decrypt(&enc).ok())
        .unwrap_or_else(|| "Task".into())
}

fn load_pending_tasks(s: &Store) -> Result<Vec<Task>> {
    use crate::domain::*;
    use chrono::DateTime;

    let mut stmt = s.conn.prepare(
        "SELECT id, category, scheduled_at, due_by, sched_days, after_task, points, status, snoozed_until, deferred_until, quiet FROM tasks WHERE status = 'Pending'"
    )?;

    let tasks = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let cat_str: String = row.get(1)?;
            let category = match cat_str.as_str() {
                "Medication" => Category::Medication,
                "Meal" => Category::Meal,
                "Care" => Category::Care,
                "Exercise" => Category::Exercise,
                "Work" => Category::Work,
                "Home" => Category::Home,
                _ => Category::Other,
            };
            let scheduled_at = row
                .get::<_, Option<String>>(2)?
                .and_then(|v| v.parse::<DateTime<Utc>>().ok());
            let due_by = row
                .get::<_, Option<String>>(3)?
                .and_then(|v| v.parse::<DateTime<Utc>>().ok());
            let days_str: String = row.get(4)?;
            use chrono::Weekday;
            let scheduled_days: Vec<Weekday> = days_str
                .split(',')
                .filter_map(|x| x.parse::<u8>().ok())
                .filter_map(|n| match n {
                    0 => Some(Weekday::Mon), 1 => Some(Weekday::Tue),
                    2 => Some(Weekday::Wed), 3 => Some(Weekday::Thu),
                    4 => Some(Weekday::Fri), 5 => Some(Weekday::Sat),
                    6 => Some(Weekday::Sun), _ => None,
                })
                .collect();
            let after_task = row.get(5)?;
            let points = row.get(6)?;
            let status_str: String = row.get(7)?;
            let status = if status_str == "Done" { Status::Done } else { Status::Pending };
            let snoozed_until = row
                .get::<_, Option<String>>(8)?
                .and_then(|v| v.parse::<DateTime<Utc>>().ok());
            let deferred_until = row
                .get::<_, Option<String>>(9)?
                .and_then(|v| v.parse::<chrono::NaiveDate>().ok());
            let quiet: bool = row.get::<_, i32>(10)? != 0;
            Ok(Task {
                id,
                title: String::new(), // title fetched separately (encrypted)
                category,
                scheduled_at,
                due_by,
                scheduled_days,
                after_task,
                points,
                status,
                snoozed_until,
                deferred_until,
                quiet,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tasks)
}

fn load_focus_mode(s: &Store) -> Result<FocusMode> {
    let v = load_setting(s, "focus_mode")?;
    Ok(match v.as_str() {
        "Essentials" => FocusMode::Essentials,
        "Quiet" => FocusMode::Quiet,
        _ => FocusMode::Normal,
    })
}

fn load_setting(s: &Store, key: &str) -> Result<String> {
    Ok(s.conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |r| r.get(0),
    )?)
}

fn load_todays_completed_ids(s: &Store) -> Result<Vec<String>> {
    let today = Utc::now().date_naive().to_string();
    let mut stmt = s.conn.prepare(
        "SELECT task_id FROM completions WHERE completed_at LIKE ?1"
    )?;
    let ids = stmt
        .query_map([format!("{today}%")], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}
