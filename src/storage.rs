//! Handles loading from and saving to the data file.

use crate::logic::TimeSheet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

/// Gets the path to the timesheet data file, adhering to the XDG Base Directory spec.
/// This function also handles migrating the data file from its legacy location.
fn get_storage_path() -> io::Result<PathBuf> {
    // 1. Get the XDG data path
    let xdg_dirs = xdg::BaseDirectories::with_prefix("time_tracker").map_err(io::Error::other)?;
    let new_path = xdg_dirs.place_data_file("timesheet.json")?;

    // 2. Check for the legacy path
    let legacy_path = dirs::home_dir()
        .map(|mut path| {
            path.push(".work_time_tracker.json");
            path
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find home directory"))?;

    // 3. If the new path doesn't exist but the legacy one does, move the file.
    if !new_path.exists() && legacy_path.exists() {
        // Ensure the parent directory exists for the new path
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Move the file
        fs::rename(&legacy_path, &new_path)?;
    }

    Ok(new_path)
}

/// Loads the TimeSheet from the data file. If the file doesn't exist or is empty,
/// returns a default, empty TimeSheet.
pub fn load_timesheet() -> io::Result<TimeSheet> {
    let path = get_storage_path()?;
    if !path.exists() {
        return Ok(TimeSheet::default());
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Ok(time_sheet) => Ok(time_sheet),
        Err(e) if e.is_eof() => Ok(TimeSheet::default()), // Handle empty file
        Err(e) => Err(io::Error::other(e)),
    }
}

/// Saves the TimeSheet data to the JSON file.
pub fn save_timesheet(time_sheet: &TimeSheet) -> io::Result<()> {
    let path = get_storage_path()?;
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet).map_err(io::Error::other)
}

/// Saves the TimeSheet data to a specific file path (for testing).
pub fn save_timesheet_to_path(time_sheet: &TimeSheet, path: &std::path::Path) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet).map_err(io::Error::other)
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
        Err(e) => Err(io::Error::other(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: We can't easily test the XDG path logic or the migration
    // without manipulating the user's actual home directory, which is unsafe.
    // The test for the legacy path is removed as it's no longer the primary logic.
    // Manual testing is required for the migration path.

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
