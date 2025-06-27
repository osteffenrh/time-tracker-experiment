//! Handles loading from and saving to the data file.

use crate::logic::TimeSheet;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

/// Gets the path to the timesheet data file (~/.work_time_tracker.json).
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
    let file = OpenOptions::new().write(true).truncate(true).create(true).open(&path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, time_sheet).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

