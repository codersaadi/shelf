use chrono::{Datelike, TimeZone};

/// A calendar-relative date window. Ranges are inclusive unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSpec {
    Today,
    Yesterday,
    /// Modified in the last 7 days (rolling).
    ThisWeek,
    /// Modified 7 to 13 days ago.
    LastWeek,
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
    /// Most recent occurrence of a month (this year, or last year if not reached yet).
    Month(u32),
    MonthYear(u32, i32),
    LastDays(u32),
    LastWeeks(u32),
    LastMonths(u32),
}

impl DateSpec {
    /// `(from, to)` as inclusive unix seconds; `to` is the present for rolling windows.
    pub fn range(&self, now: i64) -> (Option<i64>, Option<i64>) {
        let day_start = start_of_day(now);
        match self {
            DateSpec::Today => (Some(day_start), Some(now)),
            // Calendar arithmetic, not `day_start - 86_400`: that lands an hour
            // off on the DST transition days.
            DateSpec::Yesterday => (
                Some(day_start_of(&local_date(now).pred_opt().unwrap_or(chrono::NaiveDate::MIN))),
                Some(day_start - 1),
            ),
            DateSpec::ThisWeek => (Some(now - 7 * 86_400), Some(now)),
            DateSpec::LastWeek => (Some(now - 14 * 86_400), Some(now - 7 * 86_400 - 1)),
            DateSpec::ThisMonth => (Some(month_start_from(now, 0)), Some(now)),
            DateSpec::LastMonth => {
                let end = month_start_from(now, 0) - 1;
                (Some(month_start_from(end, 0)), Some(end))
            }
            DateSpec::ThisYear => {
                let year = local_date(now).year();
                (Some(year_start(year)), Some(now))
            }
            DateSpec::LastYear => {
                let year = local_date(now).year() - 1;
                (Some(year_start(year)), Some(year_start(year + 1) - 1))
            }
            DateSpec::Month(m) => {
                let start = most_recent_month_start(now, *m);
                let end = next_month_start(start) - 1;
                (Some(start), Some(end.min(now)))
            }
            DateSpec::MonthYear(m, y) => {
                let start = day_start_of(&first_day(*y, *m));
                let end = next_month_start(start) - 1;
                (Some(start), Some(end))
            }
            DateSpec::LastDays(n) => (Some(now - *n as i64 * 86_400), Some(now)),
            DateSpec::LastWeeks(n) => (Some(now - *n as i64 * 7 * 86_400), Some(now)),
            DateSpec::LastMonths(n) => (Some(month_start_from(now, *n as i64)), Some(now)),
        }
    }

    pub fn description(&self, now: i64) -> String {
        let (from, to) = self.range(now);
        match (from, to) {
            (Some(f), Some(t)) => {
                format!("Matches files modified between {} and {}", fmt_day(f), fmt_day(t))
            }
            _ => "Matches files by date".to_string(),
        }
    }
}

fn year_start(year: i32) -> i64 {
    day_start_of(&first_day(year, 1))
}

fn local_date(now: i64) -> chrono::NaiveDate {
    chrono::DateTime::from_timestamp(now, 0)
        .map(|utc| utc.with_timezone(&chrono::Local).date_naive())
        .unwrap_or_else(|| chrono::Local::now().date_naive())
}

fn start_of_day(now: i64) -> i64 {
    day_start_of(&local_date(now))
}

fn day_start_of(date: &chrono::NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0).map_or(0, to_ts)
}

/// Start of the local day `offset` calendar months before the day containing `now`.
fn month_start_from(now: i64, offset: i64) -> i64 {
    let today = local_date(now);
    let mut y = today.year();
    let mut m = today.month() as i64 - offset;
    while m < 1 {
        m += 12;
        y -= 1;
    }
    while m > 12 {
        m -= 12;
        y += 1;
    }
    day_start_of(&first_day(y, m as u32))
}

fn most_recent_month_start(now: i64, month: u32) -> i64 {
    let today = local_date(now);
    let candidate = first_day(today.year(), month);
    let start = if candidate > today { first_day(today.year() - 1, month) } else { candidate };
    day_start_of(&start)
}

fn first_day(year: i32, month: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(chrono::NaiveDate::MIN)
}

fn to_ts(dt: chrono::NaiveDateTime) -> i64 {
    match chrono::Local.from_local_datetime(&dt) {
        chrono::LocalResult::Single(t) => t.timestamp(),
        // Ambiguous (clock went back): take the earlier instant.
        chrono::LocalResult::Ambiguous(earliest, _) => earliest.timestamp(),
        // Spring-forward gap: snap forward to the first local instant that
        // exists, so "this month" never silently widens to 1970.
        chrono::LocalResult::None => (1..=2)
            .find_map(|h| {
                chrono::Local
                    .from_local_datetime(&(dt + chrono::Duration::hours(h)))
                    .single()
                    .map(|t| t.timestamp())
            })
            .unwrap_or_else(|| dt.and_utc().timestamp()),
    }
}

fn next_month_start(start_ts: i64) -> i64 {
    let d = local_date(start_ts);
    let (y, m) = if d.month() == 12 { (d.year() + 1, 1) } else { (d.year(), d.month() + 1) };
    day_start_of(&first_day(y, m))
}

fn fmt_day(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|utc| utc.with_timezone(&chrono::Local).format("%b %-d, %Y").to_string())
        .unwrap_or_default()
}
