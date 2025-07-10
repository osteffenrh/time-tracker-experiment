//! Contains the core business logic and data structures for the time tracker.

use chrono::{DateTime, Utc, Duration, Local, Datelike, NaiveDate, TimeZone, NaiveTime};
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

/// Enum to specify different ways to set time (now, offset, or absolute time today)
pub enum TimeSpec {
    Now,
    Offset(Duration),
    Absolute(NaiveTime),
}

/// Converts a TimeSpec to a DateTime<Utc>
pub fn resolve_time_spec(time_spec: &TimeSpec) -> io::Result<DateTime<Utc>> {
    match time_spec {
        TimeSpec::Now => Ok(Utc::now()),
        TimeSpec::Offset(duration) => Ok(Utc::now() + *duration),
        TimeSpec::Absolute(naive_time) => {
            let today_local = Local::now().date_naive();
            let today_datetime = today_local.and_time(*naive_time);
            naive_to_utc(today_datetime)
        }
    }
}

/// Starts a new tracking period in the timesheet.
/// Optionally uses the specified time spec, otherwise uses current time.
/// Returns an error message if a period is already active.
pub fn start_tracking(time_sheet: &mut TimeSheet, time_spec: Option<TimeSpec>) -> Result<(), &'static str> {
    if time_sheet.active_period_start.is_some() {
        Err("Already tracking time.")
    } else {
        let start_time = match time_spec {
            Some(spec) => resolve_time_spec(&spec).map_err(|_| "Invalid time specification")?,
            None => Utc::now(),
        };
        time_sheet.active_period_start = Some(start_time);
        Ok(())
    }
}

/// Stops the current tracking period.
/// Optionally uses the specified time spec, otherwise uses current time.
/// Returns the duration of the stopped period, or None if no period was active.
pub fn stop_tracking(time_sheet: &mut TimeSheet, time_spec: Option<TimeSpec>) -> Option<Duration> {
    if let Some(start_time) = time_sheet.active_period_start.take() {
        let end_time = match time_spec {
            Some(spec) => resolve_time_spec(&spec).ok()?,
            None => Utc::now(),
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_period_overlap_no_overlap() {
        let base_time = Utc::now();
        let period1 = Period {
            start: base_time,
            end: base_time + Duration::hours(1),
        };
        let period2 = Period {
            start: base_time + Duration::hours(2),
            end: base_time + Duration::hours(3),
        };
        
        assert_eq!(period1.overlap(&period2), Duration::zero());
        assert_eq!(period2.overlap(&period1), Duration::zero());
    }

    #[test]
    fn test_period_overlap_full_overlap() {
        let base_time = Utc::now();
        let period1 = Period {
            start: base_time,
            end: base_time + Duration::hours(2),
        };
        let period2 = Period {
            start: base_time + Duration::minutes(30),
            end: base_time + Duration::hours(1),
        };
        
        assert_eq!(period1.overlap(&period2), Duration::minutes(30));
        assert_eq!(period2.overlap(&period1), Duration::minutes(30));
    }

    #[test]
    fn test_period_overlap_partial_overlap() {
        let base_time = Utc::now();
        let period1 = Period {
            start: base_time,
            end: base_time + Duration::hours(2),
        };
        let period2 = Period {
            start: base_time + Duration::hours(1),
            end: base_time + Duration::hours(3),
        };
        
        assert_eq!(period1.overlap(&period2), Duration::hours(1));
        assert_eq!(period2.overlap(&period1), Duration::hours(1));
    }

    #[test]
    fn test_period_overlap_identical_periods() {
        let base_time = Utc::now();
        let period1 = Period {
            start: base_time,
            end: base_time + Duration::hours(1),
        };
        let period2 = Period {
            start: base_time,
            end: base_time + Duration::hours(1),
        };
        
        assert_eq!(period1.overlap(&period2), Duration::hours(1));
    }

    #[test]
    fn test_period_overlap_touching_periods() {
        let base_time = Utc::now();
        let period1 = Period {
            start: base_time,
            end: base_time + Duration::hours(1),
        };
        let period2 = Period {
            start: base_time + Duration::hours(1),
            end: base_time + Duration::hours(2),
        };
        
        assert_eq!(period1.overlap(&period2), Duration::zero());
    }

    #[test]
    fn test_resolve_time_spec_now() {
        let before = Utc::now();
        let result = resolve_time_spec(&TimeSpec::Now).unwrap();
        let after = Utc::now();
        
        assert!(result >= before && result <= after);
    }

    #[test]
    fn test_resolve_time_spec_offset_positive() {
        let offset = Duration::hours(1);
        let before = Utc::now() + offset;
        let result = resolve_time_spec(&TimeSpec::Offset(offset)).unwrap();
        let after = Utc::now() + offset;
        
        assert!(result >= before && result <= after);
    }

    #[test]
    fn test_resolve_time_spec_offset_negative() {
        let offset = Duration::hours(-1);
        let before = Utc::now() + offset;
        let result = resolve_time_spec(&TimeSpec::Offset(offset)).unwrap();
        let after = Utc::now() + offset;
        
        assert!(result >= before && result <= after);
    }

    #[test]
    fn test_resolve_time_spec_absolute() {
        let time = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
        let result = resolve_time_spec(&TimeSpec::Absolute(time)).unwrap();
        
        let result_local = result.with_timezone(&Local);
        assert_eq!(result_local.time(), time);
    }

    #[test]
    fn test_start_tracking_success() {
        let mut time_sheet = TimeSheet::default();
        let result = start_tracking(&mut time_sheet, None);
        
        assert!(result.is_ok());
        assert!(time_sheet.active_period_start.is_some());
    }

    #[test]
    fn test_start_tracking_already_active() {
        let mut time_sheet = TimeSheet::default();
        time_sheet.active_period_start = Some(Utc::now());
        let result = start_tracking(&mut time_sheet, None);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Already tracking time.");
    }

    #[test]
    fn test_stop_tracking_success() {
        let mut time_sheet = TimeSheet::default();
        let start_time = Utc::now();
        time_sheet.active_period_start = Some(start_time);
        
        let result = stop_tracking(&mut time_sheet, None);
        
        assert!(result.is_some());
        assert!(time_sheet.active_period_start.is_none());
        assert_eq!(time_sheet.periods.len(), 1);
        assert_eq!(time_sheet.periods[0].start, start_time);
    }

    #[test]
    fn test_stop_tracking_no_active_period() {
        let mut time_sheet = TimeSheet::default();
        let result = stop_tracking(&mut time_sheet, None);
        
        assert!(result.is_none());
        assert_eq!(time_sheet.periods.len(), 0);
    }

    #[test]
    fn test_get_today_period() {
        let result = get_today_period();
        assert!(result.is_ok());
        
        let period = result.unwrap();
        let duration = period.end - period.start;
        assert_eq!(duration, Duration::days(1));
    }

    #[test]
    fn test_get_week_period() {
        let result = get_week_period();
        assert!(result.is_ok());
        
        let period = result.unwrap();
        let duration = period.end - period.start;
        assert_eq!(duration, Duration::weeks(1));
    }

    #[test]
    fn test_get_month_period() {
        let result = get_month_period();
        assert!(result.is_ok());
        
        let period = result.unwrap();
        let duration = period.end - period.start;
        assert!(duration >= Duration::days(28) && duration <= Duration::days(31));
    }

    #[test]
    fn test_calculate_tracked_time_no_periods() {
        let time_sheet = TimeSheet::default();
        let reporting_period = Period {
            start: Utc::now(),
            end: Utc::now() + Duration::hours(8),
        };
        
        let result = calculate_tracked_time_in_period(&time_sheet, &reporting_period);
        assert_eq!(result, Duration::zero());
    }

    #[test]
    fn test_calculate_tracked_time_with_completed_periods() {
        let base_time = Utc::now();
        let mut time_sheet = TimeSheet::default();
        time_sheet.periods.push(Period {
            start: base_time,
            end: base_time + Duration::hours(2),
        });
        time_sheet.periods.push(Period {
            start: base_time + Duration::hours(3),
            end: base_time + Duration::hours(4),
        });
        
        let reporting_period = Period {
            start: base_time,
            end: base_time + Duration::hours(8),
        };
        
        let result = calculate_tracked_time_in_period(&time_sheet, &reporting_period);
        assert_eq!(result, Duration::hours(3));
    }
}

