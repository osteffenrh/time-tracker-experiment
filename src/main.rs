use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

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
        serde_json::to_writer_pretty(writer, &time_sheet)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
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
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

// Handles the "start" command. Returns true if the state was changed.
fn start_tracking(time_sheet: &mut TimeSheet) -> io::Result<bool> {
    if let Some(start_time) = time_sheet.active_period_start {
        println!(
            "Already tracking time since {}.",
            start_time.with_timezone(&chrono::Local)
        );
        Ok(false) // No change was made
    } else {
        let now = Utc::now();
        time_sheet.active_period_start = Some(now);
        println!(
            "Started tracking time at {}.",
            now.with_timezone(&chrono::Local)
        );
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
        println!(
            "Stopped tracking time at {}.",
            end_time.with_timezone(&chrono::Local)
        );
        println!("Duration of last session: {}", format_duration(duration));
        Ok(true) // A change was made
    } else {
        println!("No active time tracking period to stop.");
        Ok(false) // No change was made
    }
}

// Formats a Duration into a human-readable string (HH:MM:SS).
fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
