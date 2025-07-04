use chrono::{DateTime, Utc, Duration, Local, Datelike, NaiveDate, TimeZone};
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::env;
use std::path::PathBuf;
use std::cmp;

// Represents a single time period with a start and end time.
// Added Clone and Copy to make it easier to pass around.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct Period {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl Period {
    /// Calculates the overlapping duration between this period and another.
    fn overlap(&self, other: &Period) -> Duration {
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
struct TimeSheet {
    periods: Vec<Period>,
    active_period_start: Option<DateTime<Utc>>,
}

// Main function to parse command-line arguments and dispatch to the correct handler.
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    let mut time_sheet = load_or_create_timesheet()?;
    let mut state_changed = false;

    match command.as_str() {
        "start" => {
            state_changed = start_tracking(&mut time_sheet)?;
        }
        "stop" => {
            state_changed = stop_tracking(&mut time_sheet)?;
        }
        "today" | "week" | "month" => {
            report_summary(&time_sheet, command.as_str())?;
        }
        _ => print_usage(),
    }

    // Only save the timesheet if a change was actually made.
    if state_changed {
        save_timesheet(&time_sheet)?;
        println!("State saved.");
    }

    Ok(())
}

// Prints the usage instructions for the command-line tool.
fn print_usage() {
    println!("Usage: work_time_tracker <command>");
    println!("Commands:");
    println!("  start   - Start tracking a new time period.");
    println!("  stop    - Stop the currently tracked time period.");
    println!("  today   - Show tracked time for today.");
    println!("  week    - Show tracked time for this week.");
    println!("  month   - Show tracked time for this month.");
}

// Gets the path to the timesheet data file.
fn get_data_file_path() -> io::Result<PathBuf> {
    match dirs::home_dir() {
        Some(mut path) => {
            path.push(".work_time_tracker.json");
            Ok(path)
        }
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find home directory.",
        )),
    }
}

// Loads the TimeSheet from the data file.
fn load_or_create_timesheet() -> io::Result<TimeSheet> {
    let path = get_data_file_path()?;
    if !path.exists() {
        return Ok(TimeSheet::default());
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Ok(time_sheet) => Ok(time_sheet),
        Err(e) if e.is_eof() => Ok(TimeSheet::default()),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
    }
}

// Saves the TimeSheet data to the JSON file.
fn save_timesheet(time_sheet: &TimeSheet) -> io::Result<()> {
    let path = get_data_file_path()?;
    let file = OpenOptions::new().write(true).truncate(true).create(true).open(&path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

// Handles the "start" command.
fn start_tracking(time_sheet: &mut TimeSheet) -> io::Result<bool> {
    if let Some(start_time) = time_sheet.active_period_start {
        println!("Already tracking time since {}.", start_time.with_timezone(&Local));
        Ok(false)
    } else {
        let now = Utc::now();
        time_sheet.active_period_start = Some(now);
        println!("Started tracking time at {}.", now.with_timezone(&Local));
        Ok(true)
    }
}

// Handles the "stop" command.
fn stop_tracking(time_sheet: &mut TimeSheet) -> io::Result<bool> {
    if let Some(start_time) = time_sheet.active_period_start.take() {
        let end_time = Utc::now();
        let new_period = Period { start: start_time, end: end_time };
        time_sheet.periods.push(new_period);
        let duration = end_time - start_time;
        println!("Stopped tracking time at {}.", end_time.with_timezone(&Local));
        println!("Duration of last session: {}", format_duration(duration));
        Ok(true)
    } else {
        println!("No active time tracking period to stop.");
        Ok(false)
    }
}

/// Generates a Period struct representing the current day in the local timezone.
fn get_today_period() -> Period {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();
    let start_naive = today_local_naive.and_hms_opt(0, 0, 0).unwrap();
    let end_naive = start_naive + Duration::days(1);
    Period {
        start: Local.from_local_datetime(&start_naive).unwrap().to_utc(),
        end: Local.from_local_datetime(&end_naive).unwrap().to_utc(),
    }
}

/// Generates a Period struct representing the current week (Mon-Sun) in the local timezone.
fn get_week_period() -> Period {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();
    let days_from_monday = today_local_naive.weekday().num_days_from_monday();
    let start_of_week_naive = today_local_naive - Duration::days(days_from_monday as i64);
    let start_naive = start_of_week_naive.and_hms_opt(0, 0, 0).unwrap();
    let end_naive = start_naive + Duration::weeks(1);
    Period {
        start: Local.from_local_datetime(&start_naive).unwrap().to_utc(),
        end: Local.from_local_datetime(&end_naive).unwrap().to_utc(),
    }
}

/// Generates a Period struct representing the current month in the local timezone.
fn get_month_period() -> Period {
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
    Period {
        start: Local.from_local_datetime(&start_naive).unwrap().to_utc(),
        end: Local.from_local_datetime(&end_naive).unwrap().to_utc(),
    }
}

// Generates and prints a summary report.
fn report_summary(time_sheet: &TimeSheet, period_name: &str) -> io::Result<()> {
    let reporting_period = match period_name {
        "today" => get_today_period(),
        "week" => get_week_period(),
        "month" => get_month_period(),
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid summary period")),
    };

    let total_duration = calculate_tracked_time_in_period(time_sheet, &reporting_period);
    println!("Total time tracked for this {}: {}", period_name, format_duration(total_duration));

    Ok(())
}

// Calculates the total tracked time within a given period using iterators.
fn calculate_tracked_time_in_period(time_sheet: &TimeSheet, reporting_period: &Period) -> Duration {
    // Calculate total duration from completed periods using an iterator chain.
    let completed_duration: Duration = time_sheet.periods
        .iter()
        .map(|p| p.overlap(reporting_period))
        .sum();

    // Calculate duration from the currently active period, if any.
    let active_duration = time_sheet.active_period_start.map_or(Duration::zero(), |start| {
        let active_period = Period { start, end: Utc::now() };
        active_period.overlap(reporting_period)
    });

    completed_duration + active_duration
}

// Formats a Duration into a human-readable string (HH:MM:SS).
fn format_duration(duration: Duration) -> String {
    if duration < Duration::zero() {
        return "00:00:00".to_string();
    }
    let seconds = duration.num_seconds();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// To make this code runnable, you'll need to add the following dependencies
// to your `Cargo.toml` file:
//
// [dependencies]
// chrono = { version = "0.4", features = ["serde"] }
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// dirs = "5.0"

