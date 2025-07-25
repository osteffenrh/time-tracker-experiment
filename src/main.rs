//! The command-line interface for the work time tracker.
//! This module handles argument parsing and user-facing I/O.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use std::env;
use std::io;
use std::process::Command;
use time_tracker::{config, logic, storage};

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

    // Handle the "edit" command separately to avoid loading a potentially broken timesheet.
    if command == "edit" {
        if let Err(e) = open_in_editor() {
            eprintln!("Failed to open editor: {e}");
        }
        return Ok(());
    }

    let config = config::load_config()?;
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
        "sick" => {
            let date = if args.len() == 3 {
                match NaiveDate::parse_from_str(&args[2], "%Y-%m-%d") {
                    Ok(date) => date,
                    Err(_) => {
                        eprintln!("Invalid date format. Please use YYYY-MM-DD.");
                        return Ok(());
                    }
                }
            } else {
                chrono::Local::now().date_naive()
            };

            match logic::mark_sick_day(&mut time_sheet, date) {
                Ok(_) => {
                    println!("Marked {} as a sick day.", date.format("%Y-%m-%d"));
                    state_changed = true;
                }
                Err(msg) => eprintln!("Error: {msg}"),
            }
        }
        "today" => handle_report(&time_sheet, &config, logic::ReportingPeriod::Today, "today")?,
        "week" => handle_report(&time_sheet, &config, logic::ReportingPeriod::Week, "week")?,
        "week-breakdown" => handle_week_breakdown_report(&time_sheet, &config)?,
        "month" => handle_report(&time_sheet, &config, logic::ReportingPeriod::Month, "month")?,
        _ => print_usage(),
    }

    if state_changed {
        storage::save_timesheet(&time_sheet)?;
        println!("State saved.");
    }

    Ok(())
}

/// Opens the timesheet data file in the default editor.
fn open_in_editor() -> io::Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let path = storage::get_storage_path()?;

    let status = Command::new(editor).arg(path).status()?;

    if !status.success() {
        return Err(io::Error::other("Editor command failed"));
    }

    Ok(())
}

/// Handles generating and printing a summary report.
fn handle_report(
    time_sheet: &logic::TimeSheet,
    config: &config::Config,
    period: logic::ReportingPeriod,
    period_name: &str,
) -> io::Result<()> {
    let reporting_period = match period {
        logic::ReportingPeriod::Today => logic::get_today_period()?,
        logic::ReportingPeriod::Week => logic::get_week_period()?,
        logic::ReportingPeriod::Month => logic::get_month_period()?,
    };

    let total_duration = logic::calculate_tracked_time_in_period(time_sheet, &reporting_period);
    let target_duration =
        logic::get_target_duration_for_period(time_sheet, &config.work_hours, &reporting_period);

    let percentage = if target_duration > Duration::zero() {
        (total_duration.num_seconds() as f64 / target_duration.num_seconds() as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "Total time tracked for this {}: {} / {} ({:.2}%)",
        period_name,
        format_duration(total_duration),
        format_duration(target_duration),
        percentage
    );

    Ok(())
}

/// Handles generating and printing a week breakdown report.
fn handle_week_breakdown_report(
    time_sheet: &logic::TimeSheet,
    config: &config::Config,
) -> io::Result<()> {
    let today = chrono::Local::now().date_naive();
    let breakdown = logic::get_week_breakdown_for_date(time_sheet, today)?;

    println!("Time tracked this week (by day):");

    let mut total_duration = Duration::zero();
    let mut total_target_duration = Duration::zero();

    for (date, duration) in breakdown {
        let daily_target_hours = logic::get_target_duration_for_period(
            time_sheet,
            &config.work_hours,
            &logic::get_day_period(date)?,
        );

        if time_sheet.sick_days.contains(&date) {
            println!(
                "  {:<10} ({}): {} (sick day)",
                date.weekday(),
                date.format("%Y-%m-%d"),
                format_duration(duration)
            );
        } else {
            let percentage = if daily_target_hours > Duration::zero() {
                (duration.num_seconds() as f64 / daily_target_hours.num_seconds() as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "  {:<10} ({}): {} / {} ({:.2}%)",
                date.weekday(),
                date.format("%Y-%m-%d"),
                format_duration(duration),
                format_duration(daily_target_hours),
                percentage
            );
        }
        total_duration += duration;
        total_target_duration += daily_target_hours;
    }

    let total_percentage = if total_target_duration > Duration::zero() {
        (total_duration.num_seconds() as f64 / total_target_duration.num_seconds() as f64) * 100.0
    } else {
        0.0
    };

    println!("------------------------------------");
    println!(
        "  {:<25}: {} / {} ({:.2}%)",
        "Total for the week",
        format_duration(total_duration),
        format_duration(total_target_duration),
        total_percentage
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
    println!("  sick [date]    - Mark a date as a sick day (YYYY-MM-DD). Defaults to today.");
    println!("  edit           - Open the data file in the default editor.");
    println!("  today          - Show tracked time for today.");
    println!("  week           - Show tracked time for this week.");
    println!("  week-breakdown - Show a daily breakdown of tracked time for this week.");
    println!("  month          - Show tracked time for this month.");
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
