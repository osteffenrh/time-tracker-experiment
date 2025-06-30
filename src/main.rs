//! The command-line interface for the work time tracker.
//! This module handles argument parsing and user-facing I/O.

use chrono::Duration;
use std::env;
use std::io;
use time_tracker::{logic, storage};

// Main application entry point.
fn parse_offset(offset_str: &str) -> Option<chrono::Duration> {
    let negative = offset_str.starts_with('-');
    let clean = offset_str.trim_start_matches(['-', '+'].as_ref());
    let parts: Vec<&str> = clean.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hours: i64 = parts[0].parse().ok()?;
    let minutes: i64 = parts[1].parse().ok()?;
    let total_minutes = hours * 60 + minutes;
    let duration = chrono::Duration::minutes(total_minutes);
    Some(if negative { -duration } else { duration })
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    let mut time_sheet = storage::load_timesheet()?;
    let mut state_changed = false;

    match command.as_str() {
        "start" => {
            let offset = if args.len() == 3 {
                match parse_offset(&args[2]) {
                    Some(dur) => Some(dur),
                    None => {
                        eprintln!("Invalid offset format. Use e.g. -1:00 or +0:30");
                        return Ok(());
                    }
                }
            } else {
                None
            };
            match logic::start_tracking(&mut time_sheet, offset) {
                Ok(_) => {
                    println!("Started tracking time.");
                    state_changed = true;
                }
                Err(msg) => println!("{}", msg),
            }
        }
        "stop" => {
            let offset = if args.len() == 3 {
                match parse_offset(&args[2]) {
                    Some(dur) => Some(dur),
                    None => {
                        eprintln!("Invalid offset format. Use e.g. -1:00 or +0:30");
                        return Ok(());
                    }
                }
            } else {
                None
            };
            if let Some(duration) = logic::stop_tracking(&mut time_sheet, offset) {
                println!("Stopped tracking time.");
                println!("Duration of last session: {}", format_duration(duration));
                state_changed = true;
            } else {
                println!("No active time tracking period to stop.");
            }
        }
        "today" => handle_report(&time_sheet, logic::ReportingPeriod::Today, "today")?,
        "week" => handle_report(&time_sheet, logic::ReportingPeriod::Week, "week")?,
        "month" => handle_report(&time_sheet, logic::ReportingPeriod::Month, "month")?,
        _ => print_usage(),
    }

    if state_changed {
        storage::save_timesheet(&time_sheet)?;
        println!("State saved.");
    }

    Ok(())
}

/// Handles generating and printing a summary report.
fn handle_report(
    time_sheet: &logic::TimeSheet,
    period: logic::ReportingPeriod,
    period_name: &str,
) -> io::Result<()> {
    let reporting_period = match period {
        logic::ReportingPeriod::Today => logic::get_today_period()?,
        logic::ReportingPeriod::Week => logic::get_week_period()?,
        logic::ReportingPeriod::Month => logic::get_month_period()?,
    };

    let total_duration = logic::calculate_tracked_time_in_period(time_sheet, &reporting_period);
    println!(
        "Total time tracked for this {}: {}",
        period_name,
        format_duration(total_duration)
    );

    Ok(())
}

/// Prints the usage instructions for the command-line tool.
fn print_usage() {
    println!("Usage: work_time_tracker <command>");
    println!("Commands:");
    println!("  start   - Start tracking a new time period.");
    println!("  stop    - Stop the currently tracked time period.");
    println!("  today   - Show tracked time for today.");
    println!("  week    - Show tracked time for this week.");
    println!("  month   - Show tracked time for this month.");
}

/// Formats a Duration into a human-readable string (HH:MM:SS).
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
