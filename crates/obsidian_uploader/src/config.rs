use crate::error::{ObsidianError, Result};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub obsidian_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl Config {
    /// 環境変数から設定を読み込む
    pub fn from_env() -> Result<Self> {
        let obsidian_dir = env::var("OBSIDIAN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("crates/obsidian_uploader/obsidian"));
        
        let output_dir = env::var("OUTPUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("crates/obsidian_uploader/dist"));
        
        let config = Config {
            obsidian_dir,
            output_dir,
        };
        
        config.validate()?;
        Ok(config)
    }
    
    /// 設定値の検証
    fn validate(&self) -> Result<()> {
        if !self.obsidian_dir.exists() {
            return Err(ObsidianError::ConfigError(
                format!("Obsidian directory does not exist: {}", self.obsidian_dir.display())
            ));
        }
        
        if !self.obsidian_dir.is_dir() {
            return Err(ObsidianError::ConfigError(
                format!("Obsidian path is not a directory: {}", self.obsidian_dir.display())
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_config_validation_success() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("test_output"),
        };
        
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation_non_existent_dir() {
        let config = Config {
            obsidian_dir: PathBuf::from("/non/existent/path"),
            output_dir: PathBuf::from("test_output"),
        };
        
        assert!(config.validate().is_err());
    }
}