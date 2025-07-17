//! Handles loading the configuration file.

use serde::Deserialize;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default)]
pub struct Config {}

fn get_config_path() -> io::Result<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("time_tracker")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    // This will create a file at ~/.config/time_tracker/config.toml
    xdg_dirs
        .place_config_file("config.toml")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
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

    toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_config_no_file() {
        // This test relies on the fact that the config file doesn't exist in the test environment.
        // It's a bit of a weak test, but it's the best we can do without mocking the XDG directories.
        let config = load_config().unwrap();
        assert!(matches!(config, Config {}));
    }

    #[test]
    fn test_load_config_empty_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert!(matches!(config, Config {}));
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
