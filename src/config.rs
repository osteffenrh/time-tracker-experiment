//! Handles loading the configuration file.

use serde::Deserialize;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub work_hours: WeekdayHours,
}

#[derive(Deserialize, Debug)]
pub struct WeekdayHours {
    #[serde(default = "default_hours")]
    pub monday: f32,
    #[serde(default = "default_hours")]
    pub tuesday: f32,
    #[serde(default = "default_hours")]
    pub wednesday: f32,
    #[serde(default = "default_hours")]
    pub thursday: f32,
    #[serde(default = "default_hours")]
    pub friday: f32,
    #[serde(default = "default_weekend_hours")]
    pub saturday: f32,
    #[serde(default = "default_weekend_hours")]
    pub sunday: f32,
}

impl Default for WeekdayHours {
    fn default() -> Self {
        Self {
            monday: default_hours(),
            tuesday: default_hours(),
            wednesday: default_hours(),
            thursday: default_hours(),
            friday: default_hours(),
            saturday: default_weekend_hours(),
            sunday: default_weekend_hours(),
        }
    }
}

fn default_hours() -> f32 {
    8.0
}

fn default_weekend_hours() -> f32 {
    0.0
}

fn get_config_path() -> io::Result<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("time_tracker").map_err(io::Error::other)?;
    // This will create a file at ~/.config/time_tracker/config.toml
    xdg_dirs
        .place_config_file("config.toml")
        .map_err(io::Error::other)
}

pub fn load_config() -> io::Result<Config> {
    let path = match get_config_path() {
        Ok(path) => path,
        Err(_) => return Ok(Config::default()), // If XDG dirs are not found, use default config
    };

    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(Config::default());
    }

    toml::from_str(&content).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_config_no_file() {
        let dir = tempdir().unwrap();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }

        let config = load_config().unwrap();
        assert_eq!(config.work_hours.monday, 8.0);
        assert_eq!(config.work_hours.friday, 8.0);
        assert_eq!(config.work_hours.saturday, 0.0);
        assert_eq!(config.work_hours.sunday, 0.0);

        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    fn test_load_config_empty_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.work_hours.monday, 8.0);
        assert_eq!(config.work_hours.friday, 8.0);
        assert_eq!(config.work_hours.saturday, 0.0);
        assert_eq!(config.work_hours.sunday, 0.0);
    }

    #[test]
    fn test_load_config_partial() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let mut file = File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[work_hours]
friday = 4.5
saturday = 2.0
"#
        )
        .unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.work_hours.monday, 8.0);
        assert_eq!(config.work_hours.tuesday, 8.0);
        assert_eq!(config.work_hours.wednesday, 8.0);
        assert_eq!(config.work_hours.thursday, 8.0);
        assert_eq!(config.work_hours.friday, 4.5);
        assert_eq!(config.work_hours.saturday, 2.0);
        assert_eq!(config.work_hours.sunday, 0.0);
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let mut file = File::create(&config_path).unwrap();
        writeln!(file, "this is not toml").unwrap();

        // We can't easily test the load_config function directly because it depends on XDG paths.
        // Instead, we'll test the parsing logic directly.
        let content = fs::read_to_string(&config_path).unwrap();
        let result: Result<Config, _> = toml::from_str(&content);
        assert!(result.is_err());
    }
}
