use crate::error::{ObsidianError, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub obsidian_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // fixed paths for Obsidian and output directories
            obsidian_dir: PathBuf::from("obsidian_uploader/obsidian/Publish"),
            output_dir: PathBuf::from("obsidian_uploader/dist"),
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        let config = Self::default();
        config.validate()?;
        Ok(config)
    }

    /// validate the obisidan path is valid and exists
    fn validate(&self) -> Result<()> {
        if !self.obsidian_dir.exists() {
            return Err(ObsidianError::Config(format!(
                "Obsidian directory does not exist: {}",
                self.obsidian_dir.display()
            )));
        }

        if !self.obsidian_dir.is_dir() {
            return Err(ObsidianError::Config(format!(
                "Obsidian path is not a directory: {}",
                self.obsidian_dir.display()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use tempfile::TempDir;

    #[rstest]
    #[case::valid_dir(true, true)] // 存在するディレクトリ
    #[case::non_existent_dir(false, false)] // 存在しないパス
    fn test_config_validation(#[case] path_exists: bool, #[case] should_succeed: bool) {
        if path_exists {
            let temp_dir = TempDir::new().unwrap();
            let config = Config {
                obsidian_dir: temp_dir.path().to_path_buf(),
                output_dir: PathBuf::from("test_output"),
            };
            assert_eq!(config.validate().is_ok(), should_succeed);
        } else {
            let config = Config {
                obsidian_dir: PathBuf::from("/non/existent/path"),
                output_dir: PathBuf::from("test_output"),
            };
            assert_eq!(config.validate().is_ok(), should_succeed);
        }
    }
}
