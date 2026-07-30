//! Tauri IPC commands exposed to the frontend.

use crate::{
    domain::{Category, FocusMode, Status, Task, Timing, completion_points, goal_streak, level},
    store::Store,
};
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared state type alias
// ---------------------------------------------------------------------------
pub type StoreState = Arc<Mutex<Store>>;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub category: String,
    pub scheduled_at: Option<String>,
    pub due_by: Option<String>,
    pub scheduled_days: Vec<u8>,
    pub after_task: Option<String>,
    pub points: u32,
    pub status: String,
    pub snoozed_until: Option<String>,
    pub deferred_until: Option<String>,
    pub quiet: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsightsDto {
    pub today_pts: i64,
    pub goal_pts: i64,
    pub streak: u32,
    pub level: u32,
    pub on_time_pct: f64,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_tasks(store: tauri::State<StoreState>) -> Result<Vec<TaskDto>, String> {
    let s = store.lock().unwrap();
    let mut stmt = s
        .conn
        .prepare("SELECT id, title_enc, category, scheduled_at, due_by, sched_days, after_task, points, status, snoozed_until, deferred_until, quiet FROM tasks ORDER BY scheduled_at")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, i32>(11)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = vec![];
    for r in rows {
        let (id, title_enc, cat, sat, dby, sdays, after, pts, status, snooze, deferred, quiet) =
            r.map_err(|e| e.to_string())?;
        let title = s.decrypt(&title_enc).unwrap_or_else(|_| "[encrypted]".into());
        let scheduled_days: Vec<u8> = sdays
            .split(',')
            .filter_map(|x| x.parse().ok())
            .collect();
        out.push(TaskDto {
            id,
            title,
            category: cat,
            scheduled_at: sat,
            due_by: dby,
            scheduled_days,
            after_task: after,
            points: pts,
            status,
            snoozed_until: snooze,
            deferred_until: deferred,
            quiet: quiet != 0,
        });
    }
    Ok(out)
}

#[tauri::command]
pub fn add_task(store: tauri::State<StoreState>, dto: TaskDto) -> Result<String, String> {
    let s = store.lock().unwrap();
    let id = if dto.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        dto.id.clone()
    };
    let title_enc = s.encrypt(&dto.title).map_err(|e| e.to_string())?;
    let days = dto
        .scheduled_days
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",");
    s.conn
        .execute(
            "INSERT OR REPLACE INTO tasks (id, title_enc, category, scheduled_at, due_by, sched_days, after_task, points, status, snoozed_until, deferred_until, quiet) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![
                id, title_enc, dto.category, dto.scheduled_at, dto.due_by,
                days, dto.after_task, dto.points, dto.status,
                dto.snoozed_until, dto.deferred_until, dto.quiet as i32
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn complete_task(
    store: tauri::State<StoreState>,
    task_id: String,
    late: bool,
) -> Result<u32, String> {
    let s = store.lock().unwrap();
    let pts: u32 = s
        .conn
        .query_row("SELECT points FROM tasks WHERE id=?1", [&task_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let earned = completion_points(
        pts,
        if late { Timing::Late } else { Timing::OnTime },
    );
    s.conn
        .execute(
            "UPDATE tasks SET status='Done' WHERE id=?1",
            [&task_id],
        )
        .map_err(|e| e.to_string())?;
    let cid = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    s.conn
        .execute(
            "INSERT INTO completions (id, task_id, completed_at, points_earned, timing) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![cid, task_id, now, earned, if late { "Late" } else { "OnTime" }],
        )
        .map_err(|e| e.to_string())?;
    Ok(earned)
}

#[tauri::command]
pub fn snooze_task(
    store: tauri::State<StoreState>,
    task_id: String,
    minutes: i64,
) -> Result<(), String> {
    let until = (Utc::now() + chrono::Duration::minutes(minutes)).to_rfc3339();
    store
        .lock()
        .unwrap()
        .conn
        .execute(
            "UPDATE tasks SET snoozed_until=?1 WHERE id=?2",
            rusqlite::params![until, task_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn defer_task(
    store: tauri::State<StoreState>,
    task_id: String,
    until_date: String,
) -> Result<(), String> {
    store
        .lock()
        .unwrap()
        .conn
        .execute(
            "UPDATE tasks SET deferred_until=?1, status='Deferred' WHERE id=?2",
            rusqlite::params![until_date, task_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(store: tauri::State<StoreState>) -> Result<Vec<(String, String)>, String> {
    let s = store.lock().unwrap();
    let mut stmt = s
        .conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(
    store: tauri::State<StoreState>,
    key: String,
    value: String,
) -> Result<(), String> {
    store
        .lock()
        .unwrap()
        .conn
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_insights(store: tauri::State<StoreState>) -> Result<InsightsDto, String> {
    let s = store.lock().unwrap();
    let today = Utc::now().date_naive().to_string();
    let today_pts: i64 = s
        .conn
        .query_row(
            "SELECT COALESCE(SUM(points_earned),0) FROM completions WHERE completed_at LIKE ?1",
            [format!("{today}%")],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let goal_pts: i64 = s
        .conn
        .query_row("SELECT value FROM settings WHERE key='goal_pts'", [], |r| r.get::<_,String>(0))
        .map_err(|e| e.to_string())?
        .parse()
        .unwrap_or(30);

    // Build last 30 days hit/miss array
    let mut days = vec![];
    for offset in 0..30i64 {
        let d = (Utc::now() - chrono::Duration::days(offset)).date_naive().to_string();
        let pts: i64 = s
            .conn
            .query_row(
                "SELECT COALESCE(SUM(points_earned),0) FROM completions WHERE completed_at LIKE ?1",
                [format!("{d}%")],
                |r| r.get(0),
            )
            .unwrap_or(0);
        days.push(pts >= goal_pts);
    }
    let days_rev: Vec<bool> = days.iter().rev().copied().collect();
    let streak = goal_streak(&days_rev);

    let total_pts: u32 = s
        .conn
        .query_row("SELECT COALESCE(SUM(points_earned),0) FROM completions", [], |r| r.get(0))
        .unwrap_or(0u32);

    let (on_time_count, total_count): (i64, i64) = s
        .conn
        .query_row(
            "SELECT SUM(CASE WHEN timing='OnTime' THEN 1 ELSE 0 END), COUNT(*) FROM completions WHERE completed_at >= date('now','-30 days')",
            [],
            |r| Ok((r.get(0).unwrap_or(0), r.get(1).unwrap_or(0))),
        )
        .unwrap_or((0, 0));
    let on_time_pct = if total_count > 0 {
        on_time_count as f64 / total_count as f64 * 100.0
    } else {
        0.0
    };

    Ok(InsightsDto {
        today_pts,
        goal_pts,
        streak,
        level: level(total_pts),
        on_time_pct,
    })
}
