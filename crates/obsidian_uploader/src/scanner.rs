use crate::error::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Obsidianディレクトリ内のMarkdownファイルをスキャンする
pub fn scan_obsidian_files<P: AsRef<Path>>(obsidian_dir: P) -> Result<Vec<PathBuf>> {
    let mut markdown_files = Vec::new();
    
    for entry in WalkDir::new(obsidian_dir.as_ref()) {
        let entry = entry?;
        let path = entry.path();
        
        // .mdファイルのみを対象とする
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            // _templates/ディレクトリを除外
            if path.components().any(|component| {
                component.as_os_str() == "_templates"
            }) {
                continue;
            }
            
            markdown_files.push(path.to_path_buf());
        }
    }
    
    // ファイルパスでソート（一貫性のため）
    markdown_files.sort();
    Ok(markdown_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_scan_obsidian_files() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // テスト用ファイル構造を作成
        fs::create_dir_all(base_path.join("tech"))?;
        fs::create_dir_all(base_path.join("daily"))?;
        fs::create_dir_all(base_path.join("_templates"))?;
        
        // .mdファイルを作成
        fs::write(base_path.join("tech/article1.md"), "# Article 1")?;
        fs::write(base_path.join("daily/2025-01-01.md"), "# Daily")?;
        fs::write(base_path.join("_templates/template.md"), "# Template")?;
        fs::write(base_path.join("README.txt"), "Not markdown")?;
        
        let files = scan_obsidian_files(base_path)?;
        
        assert_eq!(files.len(), 2); // _templates/は除外される
        
        let file_names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        
        assert!(file_names.contains(&"article1.md".to_string()));
        assert!(file_names.contains(&"2025-01-01.md".to_string()));
        assert!(!file_names.contains(&"template.md".to_string()));
        
        Ok(())
    }
    
    #[test]
    fn test_scan_empty_directory() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let files = scan_obsidian_files(temp_dir.path())?;
        assert_eq!(files.len(), 0);
        Ok(())
    }
    
    #[test]
    fn test_scan_with_nested_templates() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // ネストした_templatesディレクトリを作成
        fs::create_dir_all(base_path.join("tech/_templates"))?;
        fs::create_dir_all(base_path.join("tech/rust"))?;
        
        fs::write(base_path.join("tech/_templates/tech_template.md"), "# Tech Template")?;
        fs::write(base_path.join("tech/rust/article.md"), "# Rust Article")?;
        
        let files = scan_obsidian_files(base_path)?;
        
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap() == "article.md");
        
        Ok(())
    }
}