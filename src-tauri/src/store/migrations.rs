use anyhow::Result;
use rusqlite::Connection;

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(MIGRATION_001)?;
    Ok(())
}

const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
    id           TEXT PRIMARY KEY,
    title_enc    TEXT NOT NULL,         -- AES-256-GCM encrypted
    category     TEXT NOT NULL,
    scheduled_at TEXT,                  -- ISO-8601 UTC, nullable
    due_by       TEXT,
    sched_days   TEXT NOT NULL DEFAULT '', -- comma-separated weekday numbers
    after_task   TEXT,
    points       INTEGER NOT NULL DEFAULT 10,
    status       TEXT NOT NULL DEFAULT 'Pending',
    snoozed_until TEXT,
    deferred_until TEXT,
    quiet        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS completions (
    id           TEXT PRIMARY KEY,
    task_id      TEXT NOT NULL REFERENCES tasks(id),
    completed_at TEXT NOT NULL,         -- ISO-8601 UTC
    points_earned INTEGER NOT NULL,
    timing       TEXT NOT NULL          -- 'OnTime' | 'Late'
);

CREATE TABLE IF NOT EXISTS calendars (
    id           TEXT PRIMARY KEY,
    filename_enc TEXT NOT NULL,
    imported_at  TEXT NOT NULL,
    raw_enc      TEXT NOT NULL          -- full .ics content, AES-GCM encrypted
);

CREATE TABLE IF NOT EXISTS settings (
    key          TEXT PRIMARY KEY,
    value        TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('focus_mode',    'Normal'),
    ('goal_pts',      '30'),
    ('ntfy_topic',    ''),
    ('ntfy_enabled',  '0'),
    ('quiet_start',   '23:00'),
    ('quiet_end',     '07:00'),
    ('consent_given', '0');
"#;
