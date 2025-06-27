//! The command-line interface for the work time tracker.
//! This module handles argument parsing and user-facing I/O.

use chrono::Duration;
use std::env;
use std::io;
use time_tracker::{logic, storage};

// Main application entry point.
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    let mut time_sheet = storage::load_timesheet()?;
    let mut state_changed = false;

    match command.as_str() {
        "start" => match logic::start_tracking(&mut time_sheet) {
            Ok(_) => {
                println!("Started tracking time.");
                state_changed = true;
            }
            Err(msg) => println!("{}", msg),
        },
        "stop" => {
            if let Some(duration) = logic::stop_tracking(&mut time_sheet) {
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
