mod calendar;
mod domain;

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
            let show = MenuItem::with_id(app, "show", "Open Cadence", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::with_id("cadence-tray")
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
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_status])
        .run(tauri::generate_context!())
        .expect("Cadence failed");
}

#[cfg(test)]
mod tests {
    use super::calendar::*;
    use super::domain::*;
    use chrono::{TimeZone, Utc, Weekday};

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
        assert!(t.overdue(Utc.with_ymd_and_hms(2026, 7, 30, 10, 1, 0).unwrap()));
    }

    #[test]
    fn scheduled_day() {
        let mut t = task();
        t.scheduled_days = vec![Weekday::Thu];
        assert!(t.eligible(
            Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap(),
            FocusMode::Normal,
            &[]
        ));
        t.scheduled_days = vec![Weekday::Fri];
        assert!(!t.eligible(
            Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap(),
            FocusMode::Normal,
            &[]
        ));
    }

    #[test]
    fn sequence_mode_snooze_deferral_quiet() {
        let now = Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap();
        let mut t = task();
        t.after_task = Some("first".into());
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
        assert!(t.eligible(now, FocusMode::Normal, &["first".into()]));

        t.after_task = None;
        t.category = Category::Work;
        assert!(!t.eligible(now, FocusMode::Essentials, &[]));
        t.category = Category::Medication;
        assert!(t.eligible(now, FocusMode::Essentials, &[]));

        t.snooze(now, 10);
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
        t.snoozed_until = None;

        t.defer_to(now.date_naive() + chrono::Duration::days(1));
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
        t.deferred_until = None;

        t.quiet = true;
        assert!(!t.eligible(now, FocusMode::Normal, &[]));
    }

    #[test]
    fn scoring() {
        assert_eq!(completion_points(10, Timing::OnTime), 10);
        assert_eq!(completion_points(10, Timing::Late), 5);
        assert_eq!(goal_streak(&[true, false, true, true]), 2);
        assert_eq!(level(500), 3);
    }

    #[test]
    fn calendar_datetime() {
        assert_eq!(
            parse_datetime("20260730T090000Z", None).unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 30, 9, 0, 0).unwrap()
        );
        assert!(parse_datetime("20260730T090000", Some("Europe/London")).is_ok());
    }
}
