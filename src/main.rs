//! The command-line interface for the work time tracker.
//! This module handles argument parsing and user-facing I/O.

use chrono::{Datelike, Duration, NaiveTime};
use std::env;
use std::io;
use time_tracker::{logic, storage};

// Main application entry point.

/// Parse time input - either offset (±H:MM) or absolute time (H:MM)
fn parse_time_spec(time_str: &str) -> Option<logic::TimeSpec> {
    if time_str.starts_with('+') || time_str.starts_with('-') {
        // Handle offset format
        let negative = time_str.starts_with('-');
        let clean = time_str.trim_start_matches(['-', '+'].as_ref());
        let parts: Vec<&str> = clean.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let hours: i64 = parts[0].parse().ok()?;
        let minutes: i64 = parts[1].parse().ok()?;
        let total_minutes = hours * 60 + minutes;
        let duration = Duration::minutes(total_minutes);
        let offset = if negative { -duration } else { duration };
        Some(logic::TimeSpec::Offset(offset))
    } else {
        // Handle absolute time format (H:MM)
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let hours: u32 = parts[0].parse().ok()?;
        let minutes: u32 = parts[1].parse().ok()?;
        if hours >= 24 || minutes >= 60 {
            return None;
        }

        let naive_time = NaiveTime::from_hms_opt(hours, minutes, 0)?;
        Some(logic::TimeSpec::Absolute(naive_time))
    }
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
            let time_spec = if args.len() == 3 {
                match parse_time_spec(&args[2]) {
                    Some(spec) => Some(spec),
                    None => {
                        eprintln!(
                            "Invalid time format. Use offset (e.g. -1:00, +0:30) or absolute time (e.g. 14:30)"
                        );
                        return Ok(());
                    }
                }
            } else {
                None
            };
            match logic::start_tracking(&mut time_sheet, time_spec) {
                Ok(_) => {
                    println!("Started tracking time.");
                    state_changed = true;
                }
                Err(msg) => println!("{msg}"),
            }
        }
        "stop" => {
            let time_spec = if args.len() == 3 {
                match parse_time_spec(&args[2]) {
                    Some(spec) => Some(spec),
                    None => {
                        eprintln!(
                            "Invalid time format. Use offset (e.g. -1:00, +0:30) or absolute time (e.g. 14:30)"
                        );
                        return Ok(());
                    }
                }
            } else {
                None
            };
            if let Some(duration) = logic::stop_tracking(&mut time_sheet, time_spec) {
                println!("Stopped tracking time.");
                println!("Duration of last session: {}", format_duration(duration));
                state_changed = true;
            } else {
                println!("No active time tracking period to stop.");
            }
        }
        "today" => handle_report(&time_sheet, logic::ReportingPeriod::Today, "today")?,
        "week" => handle_report(&time_sheet, logic::ReportingPeriod::Week, "week")?,
        "week-breakdown" => handle_week_breakdown_report(&time_sheet)?,
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

/// Handles generating and printing a week breakdown report.
fn handle_week_breakdown_report(time_sheet: &logic::TimeSheet) -> io::Result<()> {
    let today = chrono::Local::now().date_naive();
    let breakdown = logic::get_week_breakdown_for_date(time_sheet, today)?;

    println!("Time tracked this week (by day):");

    let mut total_duration = Duration::zero();

    for (date, duration) in breakdown {
        println!(
            "  {:<10} ({}): {}",
            date.weekday(),
            date.format("%Y-%m-%d"),
            format_duration(duration)
        );
        total_duration += duration;
    }

    println!("------------------------------------");
    println!(
        "  {:<25}: {}",
        "Total for the week",
        format_duration(total_duration)
    );

    Ok(())
}

/// Prints the usage instructions for the command-line tool.
fn print_usage() {
    println!("Usage: work_time_tracker <command>");
    println!("Commands:");
    println!("  start [time]   - Start tracking a new time period.");
    println!("  stop [time]    - Stop the currently tracked time period.");
    println!("                   Time can be offset (±H:MM) or absolute (H:MM).");
    println!("  today   - Show tracked time for today.");
    println!("  week    - Show tracked time for this week.");
    println!("  week-breakdown - Show a daily breakdown of tracked time for this week.");
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
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}
