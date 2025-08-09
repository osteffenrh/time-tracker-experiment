//! Contains the core business logic and data structures for the time tracker.

use crate::config::WeekdayHours;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    pub sick_days: Vec<NaiveDate>,
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
pub fn start_tracking(
    time_sheet: &mut TimeSheet,
    time_spec: Option<TimeSpec>,
) -> Result<(), &'static str> {
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
        let new_period = Period {
            start: start_time,
            end: end_time,
        };
        time_sheet.periods.push(new_period);
        Some(end_time - start_time)
    } else {
        None
    }
}

/// Marks a specific date as a sick day, provided it is not in the future.
pub fn mark_sick_day(time_sheet: &mut TimeSheet, date: NaiveDate) -> Result<(), &'static str> {
    if date > Local::now().date_naive() {
        return Err("Cannot mark a future date as a sick day.");
    }
    if !time_sheet.sick_days.contains(&date) {
        time_sheet.sick_days.push(date);
        time_sheet.sick_days.sort_unstable();
    }
    Ok(())
}

/// Safely converts a NaiveDateTime in the local timezone to a UTC DateTime.
fn naive_to_utc(naive_dt: chrono::NaiveDateTime) -> io::Result<DateTime<Utc>> {
    match Local.from_local_datetime(&naive_dt) {
        chrono::LocalResult::Single(dt) => Ok(dt.to_utc()),
        chrono::LocalResult::Ambiguous(dt1, dt2) => {
            let msg = format!("Ambiguous local time during conversion: {dt1} or {dt2}");
            Err(io::Error::other(msg))
        }
        chrono::LocalResult::None => {
            let msg = format!("Invalid local time during conversion: {naive_dt}");
            Err(io::Error::other(msg))
        }
    }
}

pub fn get_day_period(date: NaiveDate) -> io::Result<Period> {
    let start_naive = date.and_hms_opt(0, 0, 0).unwrap();
    let end_naive = start_naive + Duration::days(1);
    Ok(Period {
        start: naive_to_utc(start_naive)?,
        end: naive_to_utc(end_naive)?,
    })
}

/// Generates a Period struct representing the current day.
pub fn get_today_period() -> io::Result<Period> {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();
    get_day_period(today_local_naive)
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
    let start_of_month_naive =
        NaiveDate::from_ymd_opt(today_local_naive.year(), today_local_naive.month(), 1).unwrap();
    let start_naive = start_of_month_naive.and_hms_opt(0, 0, 0).unwrap();
    let (next_month_year, next_month) = if today_local_naive.month() == 12 {
        (today_local_naive.year() + 1, 1)
    } else {
        (today_local_naive.year(), today_local_naive.month() + 1)
    };
    let start_of_next_month_naive =
        NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap();
    let end_naive = start_of_next_month_naive.and_hms_opt(0, 0, 0).unwrap();
    Ok(Period {
        start: naive_to_utc(start_naive)?,
        end: naive_to_utc(end_naive)?,
    })
}

/// Calculates the total tracked time within a given period using iterators.
pub fn calculate_tracked_time_in_period(
    time_sheet: &TimeSheet,
    reporting_period: &Period,
) -> Duration {
    let completed_duration: Duration = time_sheet
        .periods
        .iter()
        .map(|p| p.overlap(reporting_period))
        .sum();

    let active_duration = time_sheet
        .active_period_start
        .map_or(Duration::zero(), |start| {
            let active_period = Period {
                start,
                end: Utc::now(),
            };
            active_period.overlap(reporting_period)
        });

    completed_duration + active_duration
}

/// Calculates the tracked time for each day of the week containing the given date.
pub fn get_week_breakdown_for_date(
    time_sheet: &TimeSheet,
    date: NaiveDate,
) -> io::Result<Vec<(NaiveDate, Duration)>> {
    let days_from_monday = date.weekday().num_days_from_monday();
    let start_of_week_naive = date - Duration::days(days_from_monday as i64);

    let mut daily_totals = Vec::new();

    for i in 0..7 {
        let current_day_naive = start_of_week_naive + Duration::days(i);
        let start_naive = current_day_naive.and_hms_opt(0, 0, 0).unwrap();
        let end_naive = start_naive + Duration::days(1);

        let reporting_period = Period {
            start: naive_to_utc(start_naive)?,
            end: naive_to_utc(end_naive)?,
        };

        let daily_duration = calculate_tracked_time_in_period(time_sheet, &reporting_period);
        daily_totals.push((current_day_naive, daily_duration));
    }

    Ok(daily_totals)
}

fn get_hours_for_day(work_hours: &WeekdayHours, day: Weekday) -> f32 {
    match day {
        Weekday::Mon => work_hours.monday,
        Weekday::Tue => work_hours.tuesday,
        Weekday::Wed => work_hours.wednesday,
        Weekday::Thu => work_hours.thursday,
        Weekday::Fri => work_hours.friday,
        Weekday::Sat => work_hours.saturday,
        Weekday::Sun => work_hours.sunday,
    }
}

/// Calculates the total target work duration for a given period based on configured work hours.
pub fn get_target_duration_for_period(
    time_sheet: &TimeSheet,
    work_hours: &WeekdayHours,
    reporting_period: &Period,
) -> Duration {
    let mut total_hours = 0.0;
    let mut current_day = reporting_period.start.with_timezone(&Local).date_naive();
    let end_day = reporting_period.end.with_timezone(&Local).date_naive();
    let today = Local::now().date_naive();

    while current_day < end_day {
        if current_day > today {
            break;
        }
        if !time_sheet.sick_days.contains(&current_day) {
            total_hours += get_hours_for_day(work_hours, current_day.weekday());
        }
        current_day += Duration::days(1);
    }

    Duration::seconds((total_hours * 3600.0) as i64)
}

/// Formats a Duration into a human-readable string (HH:MM:SS).
pub fn format_duration(duration: Duration) -> String {
    if duration < Duration::zero() {
        return "00:00:00".to_string();
    }
    let seconds = duration.num_seconds();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(Duration::zero()), "00:00:00");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::seconds(45)), "00:00:45");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::minutes(15)), "00:15:00");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(Duration::hours(3)), "03:00:00");
    }

    #[test]
    fn test_format_duration_mixed() {
        assert_eq!(
            format_duration(Duration::hours(1) + Duration::minutes(30) + Duration::seconds(5)),
            "01:30:05"
        );
    }

    #[test]
    fn test_format_duration_negative() {
        assert_eq!(format_duration(Duration::hours(-1)), "00:00:00");
    }

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
        let mut time_sheet = TimeSheet {
            active_period_start: Some(Utc::now()),
            ..Default::default()
        };
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

    #[test]
    fn test_get_week_breakdown_for_date() {
        let mut time_sheet = TimeSheet::default();

        // Test week is Mon 16th Oct 2023 to Sun 22nd Oct 2023.
        // We'll use Wednesday 18th as the "today" for the test.
        let test_date = NaiveDate::from_ymd_opt(2023, 10, 18).unwrap();

        // Period on Monday (16th) for 2 hours.
        let monday_start = NaiveDate::from_ymd_opt(2023, 10, 16)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        time_sheet.periods.push(Period {
            start: naive_to_utc(monday_start).unwrap(),
            end: naive_to_utc(monday_start).unwrap() + Duration::hours(2),
        });

        // Period on Wednesday (18th) for 3 hours.
        let wednesday_start = NaiveDate::from_ymd_opt(2023, 10, 18)
            .unwrap()
            .and_hms_opt(13, 0, 0)
            .unwrap();
        time_sheet.periods.push(Period {
            start: naive_to_utc(wednesday_start).unwrap(),
            end: naive_to_utc(wednesday_start).unwrap() + Duration::hours(3),
        });

        let breakdown = get_week_breakdown_for_date(&time_sheet, test_date).unwrap();

        assert_eq!(breakdown.len(), 7);

        // Monday
        assert_eq!(
            breakdown[0].0,
            NaiveDate::from_ymd_opt(2023, 10, 16).unwrap()
        );
        assert_eq!(breakdown[0].1, Duration::hours(2));

        // Tuesday
        assert_eq!(
            breakdown[1].0,
            NaiveDate::from_ymd_opt(2023, 10, 17).unwrap()
        );
        assert_eq!(breakdown[1].1, Duration::zero());

        // Wednesday
        assert_eq!(
            breakdown[2].0,
            NaiveDate::from_ymd_opt(2023, 10, 18).unwrap()
        );
        assert_eq!(breakdown[2].1, Duration::hours(3));

        // Thursday
        assert_eq!(
            breakdown[3].0,
            NaiveDate::from_ymd_opt(2023, 10, 19).unwrap()
        );
        assert_eq!(breakdown[3].1, Duration::zero());

        // Rest of the week
        assert_eq!(breakdown[4].1, Duration::zero());
        assert_eq!(breakdown[5].1, Duration::zero());
        assert_eq!(breakdown[6].1, Duration::zero());
    }

    #[test]
    fn test_get_target_duration_for_period_week_in_progress() {
        let work_hours = WeekdayHours {
            monday: 8.0,
            tuesday: 8.0,
            wednesday: 8.0,
            thursday: 8.0,
            friday: 8.0,
            saturday: 0.0,
            sunday: 0.0,
        };
        let time_sheet = TimeSheet::default();

        let reporting_period = get_week_period().unwrap();
        let today = Local::now().date_naive();
        let days_from_monday = today.weekday().num_days_from_monday();

        // Calculate expected hours based on the current day of the week
        let expected_hours = if days_from_monday >= 5 {
            // Full week from Monday to Friday
            5.0 * 8.0
        } else {
            (days_from_monday as f32 + 1.0) * 8.0
        };

        let result = get_target_duration_for_period(&time_sheet, &work_hours, &reporting_period);
        assert_eq!(result, Duration::hours(expected_hours as i64));
    }

    #[test]
    fn test_mark_sick_day() {
        let mut time_sheet = TimeSheet::default();
        let today = Local::now().date_naive();
        let yesterday = today - Duration::days(1);

        assert!(mark_sick_day(&mut time_sheet, today).is_ok());
        assert_eq!(time_sheet.sick_days, vec![today]);

        // Mark the same day again, should not add a duplicate
        assert!(mark_sick_day(&mut time_sheet, today).is_ok());
        assert_eq!(time_sheet.sick_days, vec![today]);

        assert!(mark_sick_day(&mut time_sheet, yesterday).is_ok());
        assert_eq!(time_sheet.sick_days, vec![yesterday, today]); // Should be sorted
    }

    #[test]
    fn test_mark_sick_day_future_date() {
        let mut time_sheet = TimeSheet::default();
        let tomorrow = Local::now().date_naive() + Duration::days(1);
        assert!(mark_sick_day(&mut time_sheet, tomorrow).is_err());
    }

    #[test]
    fn test_get_target_duration_with_sick_day() {
        let work_hours = WeekdayHours {
            monday: 8.0,
            tuesday: 8.0,
            wednesday: 8.0,
            thursday: 8.0,
            friday: 8.0,
            saturday: 4.0,
            sunday: 0.0,
        };
        let mut time_sheet = TimeSheet::default();
        let today = Local::now().date_naive();
        let reporting_period = get_week_period().unwrap();

        // Mark a weekday as a sick day
        let sick_day =
            reporting_period.start.with_timezone(&Local).date_naive() + Duration::days(2); // Wednesday
        mark_sick_day(&mut time_sheet, sick_day).unwrap();

        let days_from_monday = today.weekday().num_days_from_monday();
        let mut expected_hours = if days_from_monday >= 5 {
            5.0 * 8.0 + 4.0 // Full week + Saturday
        } else {
            (days_from_monday as f32 + 1.0) * 8.0
        };

        // If the sick day has passed, subtract its hours
        if today.weekday().num_days_from_monday() >= 2 {
            expected_hours -= 8.0;
        }

        let result = get_target_duration_for_period(&time_sheet, &work_hours, &reporting_period);
        assert_eq!(result, Duration::hours(expected_hours as i64));
    }
}
