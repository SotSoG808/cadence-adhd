# Manual acceptance tests

Run `npm install` then `npm run dev` on clean Windows 11.

## Today
- Verify greeting, Now, Up next, Done today, points, goal/streak and Quick add.
- Mark each sample task Done; it disappears and the score updates.
- Confirm a snoozed task is absent until its expiry; a deferred task is absent until its selected date.

## Routine and modes
- Open Routine; add fixed-time, flexible-block and chained task definitions.
- Switch Normal, Essentials and Quiet modes; only medication, meals and care appear in Essentials; Quiet shows no routine alerts.
- Complete the predecessor; verify its chained successor becomes eligible.

## Calendars
- Import every fixture under `tests/fixtures`; reimport the same filename and verify it refreshes rather than duplicates.
- Verify weekly recurrence excludes EXDATE; verify UTC, TZID Europe/London and floating values display at expected local times.
- Configure event lead time and verify quiet-hour events do not alert until quiet ends.

## Insights and settings
- Verify point totals, late half-points, daily-goal streak, level and on-time rate after completions.
- Verify all navigation entries: Today, Routine, Insights, Calendars, Settings.

## Background reminders
- Close the main window (do not select Quit); verify Cadence remains in the Windows system tray.
- Click tray icon or Open Cadence to restore the window; select Quit to end process.
- Trigger an eligible reminder and verify Windows toast; test Snooze and Dismiss actions when notification permission is granted.
