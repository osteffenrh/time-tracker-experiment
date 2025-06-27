//! Contains the core business logic and data structures for the time tracker.

use chrono::{DateTime, Utc, Duration, Local, Datelike, NaiveDate, TimeZone, Weekday};
use serde::{Serialize, Deserialize};
use std::cmp;
use std::io;

// Represents a single time period with a start and end time.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Period {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl Period {
    /// Calculates the overlapping duration between this period and another.
    pub fn overlap(&self, other: &Period) -> Duration {
        let overlap_start = cmp::max(self.start, other.start);
        let overlap_end = cmp::min(self.end, other.end);

        if overlap_start < overlap_end {
            overlap_end - overlap_start
        } else {
            Duration::zero()
        }
    }
}

// Represents the overall state of the time tracker.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TimeSheet {
    pub periods: Vec<Period>,
    pub active_period_start: Option<DateTime<Utc>>,
}

/// Enum to provide compile-time safety for selecting a reporting interval.
pub enum ReportingPeriod {
    Today,
    Week,
    Month,
}

/// Starts a new tracking period in the timesheet.
/// Returns an error message if a period is already active.
pub fn start_tracking(time_sheet: &mut TimeSheet) -> Result<(), &'static str> {
    if time_sheet.active_period_start.is_some() {
        Err("Already tracking time.")
    } else {
        time_sheet.active_period_start = Some(Utc::now());
        Ok(())
    }
}

/// Stops the current tracking period.
/// Returns the duration of the stopped period, or None if no period was active.
pub fn stop_tracking(time_sheet: &mut TimeSheet) -> Option<Duration> {
    if let Some(start_time) = time_sheet.active_period_start.take() {
        let end_time = Utc::now();
        let new_period = Period { start: start_time, end: end_time };
        time_sheet.periods.push(new_period);
        Some(end_time - start_time)
    } else {
        None
    }
}

/// Safely converts a NaiveDateTime in the local timezone to a UTC DateTime.
fn naive_to_utc(naive_dt: chrono::NaiveDateTime) -> io::Result<DateTime<Utc>> {
    match Local.from_local_datetime(&naive_dt) {
        chrono::LocalResult::Single(dt) => Ok(dt.to_utc()),
        chrono::LocalResult::Ambiguous(dt1, dt2) => {
            let msg = format!("Ambiguous local time during conversion: {} or {}", dt1, dt2);
            Err(io::Error::new(io::ErrorKind::Other, msg))
        },
        chrono::LocalResult::None => {
            let msg = format!("Invalid local time during conversion: {}", naive_dt);
            Err(io::Error::new(io::ErrorKind::Other, msg))
        }
    }
}

/// Generates a Period struct representing the current day.
pub fn get_today_period() -> io::Result<Period> {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();
    let start_naive = today_local_naive.and_hms_opt(0, 0, 0).unwrap();
    let end_naive = start_naive + Duration::days(1);
    Ok(Period {
        start: naive_to_utc(start_naive)?,
        end: naive_to_utc(end_naive)?,
    })
}

/// Generates a Period struct representing the current week.
pub fn get_week_period() -> io::Result<Period> {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();
    let days_from_monday = today_local_naive.weekday().num_days_from_monday();
    let start_of_week_naive = today_local_naive - Duration::days(days_from_monday as i64);
    let start_naive = start_of_week_naive.and_hms_opt(0, 0, 0).unwrap();
    let end_naive = start_naive + Duration::weeks(1);
    Ok(Period {
        start: naive_to_utc(start_naive)?,
        end: naive_to_utc(end_naive)?,
    })
}

/// Generates a Period struct representing the current month.
pub fn get_month_period() -> io::Result<Period> {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();
    let start_of_month_naive = NaiveDate::from_ymd_opt(today_local_naive.year(), today_local_naive.month(), 1).unwrap();
    let start_naive = start_of_month_naive.and_hms_opt(0, 0, 0).unwrap();
    let (next_month_year, next_month) = if today_local_naive.month() == 12 {
        (today_local_naive.year() + 1, 1)
    } else {
        (today_local_naive.year(), today_local_naive.month() + 1)
    };
    let start_of_next_month_naive = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap();
    let end_naive = start_of_next_month_naive.and_hms_opt(0, 0, 0).unwrap();
    Ok(Period {
        start: naive_to_utc(start_naive)?,
        end: naive_to_utc(end_naive)?,
    })
}

/// Calculates the total tracked time within a given period using iterators.
pub fn calculate_tracked_time_in_period(time_sheet: &TimeSheet, reporting_period: &Period) -> Duration {
    let completed_duration: Duration = time_sheet.periods
        .iter()
        .map(|p| p.overlap(reporting_period))
        .sum();

    let active_duration = time_sheet.active_period_start.map_or(Duration::zero(), |start| {
        let active_period = Period { start, end: Utc::now() };
        active_period.overlap(reporting_period)
    });

    completed_duration + active_duration
}

