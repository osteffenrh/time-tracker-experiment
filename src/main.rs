use chrono::{DateTime, Utc, Duration, Local, Datelike, Weekday, NaiveDate, NaiveDateTime, TimeZone};
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::env;
use std::path::PathBuf;
use std::cmp;

// Represents a single completed work period with a start and end time.
#[derive(Serialize, Deserialize, Debug)]
struct Period {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

// Represents the overall state of the time tracker, including
// completed periods and any currently active period.
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
// It's located at ~/.work_time_tracker.json
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

// Loads the TimeSheet from the data file. If the file doesn't exist,
// it creates an empty one.
fn load_or_create_timesheet() -> io::Result<TimeSheet> {
    let path = get_data_file_path()?;
    if !path.exists() {
        let file = File::create(&path)?;
        let time_sheet = TimeSheet::default();
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &time_sheet).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        return Ok(time_sheet);
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Ok(time_sheet) => Ok(time_sheet),
        Err(e) if e.is_eof() => Ok(TimeSheet::default()), // Handle empty file case
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
    }
}

// Saves the TimeSheet data to the JSON file.
fn save_timesheet(time_sheet: &TimeSheet) -> io::Result<()> {
    let path = get_data_file_path()?;
    let file = OpenOptions::new().write(true).truncate(true).create(true).open(&path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

// Handles the "start" command. Returns true if the state was changed.
fn start_tracking(time_sheet: &mut TimeSheet) -> io::Result<bool> {
    if let Some(start_time) = time_sheet.active_period_start {
        println!("Already tracking time since {}.", start_time.with_timezone(&Local));
        Ok(false) // No change was made
    } else {
        let now = Utc::now();
        time_sheet.active_period_start = Some(now);
        println!("Started tracking time at {}.", now.with_timezone(&Local));
        Ok(true) // A change was made
    }
}

// Handles the "stop" command. Returns true if the state was changed.
fn stop_tracking(time_sheet: &mut TimeSheet) -> io::Result<bool> {
    if let Some(start_time) = time_sheet.active_period_start.take() {
        let end_time = Utc::now();
        let new_period = Period {
            start: start_time,
            end: end_time,
        };
        time_sheet.periods.push(new_period);
        let duration = end_time - start_time;
        println!("Stopped tracking time at {}.", end_time.with_timezone(&Local));
        println!("Duration of last session: {}", format_duration(duration));
        Ok(true) // A change was made
    } else {
        println!("No active time tracking period to stop.");
        Ok(false) // No change was made
    }
}

// Generates and prints a summary report for a given period (today, week, month).
fn report_summary(time_sheet: &TimeSheet, period_name: &str) -> io::Result<()> {
    let now_local = Local::now();
    let today_local_naive = now_local.date_naive();

    let (start_naive, end_naive) = match period_name {
        "today" => {
            let start = today_local_naive.and_hms_opt(0, 0, 0).unwrap();
            (start, start + Duration::days(1))
        }
        "week" => {
            let days_from_monday = today_local_naive.weekday().num_days_from_monday();
            let start_of_week = today_local_naive - Duration::days(days_from_monday as i64);
            let start = start_of_week.and_hms_opt(0, 0, 0).unwrap();
            (start, start + Duration::weeks(1))
        }
        "month" => {
            let start_of_month = NaiveDate::from_ymd_opt(today_local_naive.year(), today_local_naive.month(), 1).unwrap();
            let start = start_of_month.and_hms_opt(0, 0, 0).unwrap();
            
            let (next_month_year, next_month) = if today_local_naive.month() == 12 {
                (today_local_naive.year() + 1, 1)
            } else {
                (today_local_naive.year(), today_local_naive.month() + 1)
            };
            let start_of_next_month = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap();
            let end = start_of_next_month.and_hms_opt(0, 0, 0).unwrap();
            (start, end)
        }
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid summary period")),
    };

    // Correctly convert the local NaiveDateTime to a UTC DateTime.
    let start_interval_utc = Local.from_local_datetime(&start_naive).unwrap().to_utc();
    let end_interval_utc = Local.from_local_datetime(&end_naive).unwrap().to_utc();

    let total_duration = calculate_tracked_time_in_interval(time_sheet, start_interval_utc, end_interval_utc);
    println!("Total time tracked for this {}: {}", period_name, format_duration(total_duration));

    Ok(())
}

// Calculates the total tracked time within a given UTC interval,
// including completed periods and the currently active one.
fn calculate_tracked_time_in_interval(
    time_sheet: &TimeSheet,
    interval_start: DateTime<Utc>,
    interval_end: DateTime<Utc>,
) -> Duration {
    let mut total_duration = Duration::zero();

    // Sum up durations from completed periods that overlap with the interval
    for period in &time_sheet.periods {
        let overlap_start = cmp::max(period.start, interval_start);
        let overlap_end = cmp::min(period.end, interval_end);

        if overlap_start < overlap_end {
            total_duration = total_duration + (overlap_end - overlap_start);
        }
    }

    // If there's an active period, calculate its overlap with the interval
    if let Some(active_start) = time_sheet.active_period_start {
        let now_utc = Utc::now();
        let overlap_start = cmp::max(active_start, interval_start);
        let overlap_end = cmp::min(now_utc, interval_end);

        if overlap_start < overlap_end {
            total_duration = total_duration + (overlap_end - overlap_start);
        }
    }

    total_duration
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

