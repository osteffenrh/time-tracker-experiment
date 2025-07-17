//! Handles loading from and saving to the data file.

use crate::logic::TimeSheet;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

/// Gets the path to the timesheet data file (~/.work_time_tracker.json).
pub fn get_data_file_path() -> io::Result<PathBuf> {
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

/// Loads the TimeSheet from the data file. If the file doesn't exist or is empty,
/// returns a default, empty TimeSheet.
pub fn load_timesheet() -> io::Result<TimeSheet> {
    let path = get_data_file_path()?;
    if !path.exists() {
        return Ok(TimeSheet::default());
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Ok(time_sheet) => Ok(time_sheet),
        Err(e) if e.is_eof() => Ok(TimeSheet::default()), // Handle empty file
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
    }
}

/// Saves the TimeSheet data to the JSON file.
pub fn save_timesheet(time_sheet: &TimeSheet) -> io::Result<()> {
    let path = get_data_file_path()?;
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Saves the TimeSheet data to a specific file path (for testing).
pub fn save_timesheet_to_path(time_sheet: &TimeSheet, path: &std::path::Path) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Loads a TimeSheet from a specific file path (for testing).
pub fn load_timesheet_from_path(path: &std::path::Path) -> io::Result<TimeSheet> {
    if !path.exists() {
        return Ok(TimeSheet::default());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Ok(time_sheet) => Ok(time_sheet),
        Err(e) if e.is_eof() => Ok(TimeSheet::default()),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_file_path_success() {
        let result = get_data_file_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.file_name().unwrap() == ".work_time_tracker.json");
        assert!(path.is_absolute());
    }

    #[test]
    fn test_load_timesheet_nonexistent_file() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        let result = load_timesheet_from_path(temp_path);

        assert!(result.is_ok());
        let timesheet = result.unwrap();
        assert_eq!(timesheet.periods.len(), 0);
        assert!(timesheet.active_period_start.is_none());
    }

    #[test]
    fn test_save_and_load_timesheet() {
        use crate::logic::{Period, TimeSheet};
        use chrono::{Duration, Utc};
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        let mut original_timesheet = TimeSheet::default();
        original_timesheet.periods.push(Period {
            start: Utc::now(),
            end: Utc::now() + Duration::hours(1),
        });
        original_timesheet.active_period_start = Some(Utc::now());

        let save_result = save_timesheet_to_path(&original_timesheet, temp_path);
        assert!(save_result.is_ok());

        let load_result = load_timesheet_from_path(temp_path);
        assert!(load_result.is_ok());

        let loaded_timesheet = load_result.unwrap();
        assert_eq!(
            loaded_timesheet.periods.len(),
            original_timesheet.periods.len()
        );
        assert!(loaded_timesheet.active_period_start.is_some());
    }
}
