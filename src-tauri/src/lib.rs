mod calendar;
mod commands;
mod domain;
mod engine;
mod notification;
mod store;

use commands::StoreState;
use engine::{run_loop, EngineState};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[tauri::command]
fn app_status() -> &'static str {
    "Cadence reminder engine is running"
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // ----------------------------------------------------------------
            // Open encrypted store
            // ----------------------------------------------------------------
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("could not resolve app data dir");
            let store = store::Store::open(&data_dir)
                .expect("failed to open Cadence database");
            let store: StoreState = Arc::new(Mutex::new(store));
            app.manage(store.clone());

            // ----------------------------------------------------------------
            // Background reminder engine (survives window close)
            // ----------------------------------------------------------------
            let engine_state = Arc::new(Mutex::new(EngineState {
                last_reminded: HashMap::new(),
            }));
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(run_loop(app_handle, store, engine_state));

            // ----------------------------------------------------------------
            // System tray
            // ----------------------------------------------------------------
            let show = MenuItem::with_id(app, "show", "Open Cadence", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::with_id("cadence-tray")
                .tooltip("Cadence")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            commands::get_tasks,
            commands::add_task,
            commands::complete_task,
            commands::snooze_task,
            commands::defer_task,
            commands::get_settings,
            commands::set_setting,
            commands::get_insights,
        ])
        .on_window_event(|window, event| {
            // Keep process alive when window is closed; tray remains
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("Cadence failed");
}

// ---------------------------------------------------------------------------
// Automated tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::calendar::*;
    use super::domain::*;
    use chrono::{NaiveTime, TimeZone, Utc, Weekday};

    fn task() -> Task {
        Task {
            id: "a".into(),
            title: "x".into(),
            category: Category::Work,
            scheduled_at: Some(Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap()),
            due_by: Some(Utc.with_ymd_and_hms(2026, 7, 30, 10, 0, 0).unwrap()),
            scheduled_days: vec![],
            after_task: None,
            points: 10,
            status: Status::Pending,
            snoozed_until: None,
            deferred_until: None,
            quiet: false,
        }
    }

    #[test]
    fn due_overdue() {
        let t = task();
        assert!(t.due(Utc.with_ymd_and_hms(2026, 7, 30, 9, 1, 0).unwrap()));
        assert!(!t.due(Utc.with_ymd_and_hms(2026, 7, 30, 8, 59, 0).unwrap()));
        assert!(t.overdue(Utc.with_ymd_and_hms(2026, 7, 30, 10, 1, 0).unwrap()));
        assert!(!t.overdue(Utc.with_ymd_and_hms(2026, 7, 30, 9, 59, 0).unwrap()));
    }

    #[test]
    fn scheduled_day() {
        let mut t = task();
        // 2026-07-30 is a Thursday (Weekday::Thu)
        t.scheduled_days = vec![Weekday::Thu];
        assert!(t.eligible(Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap(), FocusMode::Normal, &[]));
        t.scheduled_days = vec![Weekday::Fri];
        assert!(!t.eligible(Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap(), FocusMode::Normal, &[]));
    }

    #[test]
    fn sequence() {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap();
        let mut t = task();
        t.after_task = Some("first".into());
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
        assert!(t.eligible(now, FocusMode::Normal, &["first".into()]));
    }

    #[test]
    fn focus_modes() {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap();
        let mut t = task();
        t.category = Category::Work;
        assert!(!t.eligible(now, FocusMode::Essentials, &[]));
        assert!(!t.eligible(now, FocusMode::Quiet, &[]));
        t.category = Category::Medication;
        assert!(t.eligible(now, FocusMode::Essentials, &[]));
        assert!(!t.eligible(now, FocusMode::Quiet, &[]));
    }

    #[test]
    fn snooze() {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap();
        let mut t = task();
        t.snooze(now, 10);
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
        // After snooze expires
        let later = Utc.with_ymd_and_hms(2026, 7, 30, 9, 11, 0).unwrap();
        assert!(t.eligible(later, FocusMode::Normal, &[]));
    }

    #[test]
    fn deferral() {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap();
        let mut t = task();
        t.defer_to(now.date_naive() + chrono::Duration::days(1));
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
        let tomorrow = Utc.with_ymd_and_hms(2026, 7, 31, 9, 0, 0).unwrap();
        // deferred_until is tomorrow; eligible on tomorrow
        assert!(t.eligible(tomorrow, FocusMode::Normal, &[]));
    }

    #[test]
    fn quiet_flag() {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap();
        let mut t = task();
        t.quiet = true;
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
    }

    #[test]
    fn event_quiet_hours() {
        let event_start = Utc.with_ymd_and_hms(2026, 7, 30, 23, 30, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 22, 0, 0).unwrap();
        // quiet 23:00-07:00 BST = 22:00-06:00 UTC
        let quiet = (
            NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        );
        assert!(event_is_quiet(event_start, now, quiet));
    }

    #[test]
    fn scoring() {
        assert_eq!(completion_points(10, Timing::OnTime), 10);
        assert_eq!(completion_points(10, Timing::Late), 5);
        assert_eq!(completion_points(0, Timing::OnTime), 0);
        assert_eq!(goal_streak(&[true, false, true, true]), 2);
        assert_eq!(goal_streak(&[true, true, true]), 3);
        assert_eq!(goal_streak(&[false, false]), 0);
        assert_eq!(level(0), 1);
        assert_eq!(level(249), 1);
        assert_eq!(level(250), 2);
        assert_eq!(level(500), 3);
    }

    #[test]
    fn calendar_utc() {
        let dt = parse_datetime("20260730T090000Z", None).unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap());
    }

    #[test]
    fn calendar_tzid() {
        // Europe/London is UTC+1 in summer (BST)
        let dt = parse_datetime("20260730T100000", Some("Europe/London")).unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap());
    }

    #[test]
    fn calendar_floating() {
        // floating = treated as UTC
        let dt = parse_datetime("20260730T090000", None).unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap());
    }

    #[test]
    fn calendar_exdate_weekly() {
        let event = super::calendar::CalendarEvent {
            uid: "e1".into(),
            title: "Weekly".into(),
            start: Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap(),
            exdates: vec![Utc.with_ymd_and_hms(2026, 8, 6, 9, 0, 0).unwrap()],
            rrule: Some("FREQ=WEEKLY;COUNT=4".into()),
        };
        let from = Utc.with_ymd_and_hms(2026, 7, 30, 0, 0, 0).unwrap();
        let to   = Utc.with_ymd_and_hms(2026, 8, 20, 0, 0, 0).unwrap();
        let occ = weekly_occurrences(&event, from, to);
        // 4 weekly occurrences minus 1 EXDATE = 3
        assert_eq!(occ.len(), 3);
        assert!(!occ.contains(&Utc.with_ymd_and_hms(2026, 8, 6, 9, 0, 0).unwrap()));
    }
}
