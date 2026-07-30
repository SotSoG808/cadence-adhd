use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusMode {
    Normal,
    Essentials,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Medication,
    Meal,
    Care,
    Exercise,
    Work,
    Home,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Done,
    Deferred,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub category: Category,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub due_by: Option<DateTime<Utc>>,
    pub scheduled_days: Vec<Weekday>,
    pub after_task: Option<String>,
    pub points: u32,
    pub status: Status,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub deferred_until: Option<NaiveDate>,
    pub quiet: bool,
}

impl Task {
    pub fn eligible(&self, now: DateTime<Utc>, mode: FocusMode, completed: &[String]) -> bool {
        if self.status != Status::Pending
            || self.quiet
            || self.deferred_until.is_some_and(|d| d > now.date_naive())
        {
            return false;
        }

        if !self.scheduled_days.is_empty() && !self.scheduled_days.contains(&now.weekday()) {
            return false;
        }

        if let Some(parent) = &self.after_task {
            if !completed.contains(parent) {
                return false;
            }
        }

        if self.snoozed_until.is_some_and(|t| t > now) {
            return false;
        }

        match mode {
            FocusMode::Normal => true,
            FocusMode::Essentials => {
                matches!(self.category, Category::Medication | Category::Meal | Category::Care)
            }
            FocusMode::Quiet => false,
        }
    }

    pub fn due(&self, now: DateTime<Utc>) -> bool {
        self.eligible(now, FocusMode::Normal, &[]) && self.scheduled_at.is_some_and(|t| t <= now)
    }

    pub fn overdue(&self, now: DateTime<Utc>) -> bool {
        self.status == Status::Pending && self.due_by.is_some_and(|t| t < now)
    }

    pub fn snooze(&mut self, now: DateTime<Utc>, minutes: i64) {
        self.snoozed_until = Some(now + Duration::minutes(minutes));
    }

    pub fn defer_to(&mut self, date: NaiveDate) {
        self.deferred_until = Some(date);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    OnTime,
    Late,
}

pub fn completion_points(base: u32, timing: Timing) -> u32 {
    match timing {
        Timing::OnTime => base,
        Timing::Late => base / 2,
    }
}

pub fn goal_streak(days: &[bool]) -> u32 {
    days.iter().rev().take_while(|x| **x).count() as u32
}

pub fn level(total: u32) -> u32 {
    1 + total / 250
}

pub fn event_is_quiet(
    event_start: DateTime<Utc>,
    now: DateTime<Utc>,
    quiet_hours: (NaiveTime, NaiveTime),
) -> bool {
    let local = event_start.with_timezone(&Local);
    let t = local.time();
    let in_quiet = if quiet_hours.0 <= quiet_hours.1 {
        t >= quiet_hours.0 && t < quiet_hours.1
    } else {
        t >= quiet_hours.0 || t < quiet_hours.1
    };
    in_quiet && event_start > now
}
