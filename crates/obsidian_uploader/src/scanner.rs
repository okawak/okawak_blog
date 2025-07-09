use crate::error::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// ObsidianのPublishディレクトリ内のMarkdownファイルをスキャンする
pub fn scan_obsidian_files<P: AsRef<Path>>(publish_dir: P) -> Result<Vec<PathBuf>> {
    let mut markdown_files: Vec<PathBuf> = WalkDir::new(publish_dir.as_ref())
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            path.extension()
                .and_then(|ext| ext.to_str())
                .filter(|ext| ext.to_lowercase() == "md")
                .map(|_| path.to_path_buf())
        })
        .collect();

    // ファイルパスでソート（一貫性のため、パフォーマンス向上のためsort_unstableを使用）
    markdown_files.sort_unstable();
    Ok(markdown_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::fs;
    use tempfile::TempDir;

    #[rstest]
    #[case::with_files(vec!["tech/article1.md", "daily/2025-01-01.md"], vec!["README.txt"], 2)]
    #[case::empty_dir(vec![], vec![], 0)]
    #[case::non_md_files(vec![], vec!["README.txt", "config.json"], 0)]
    fn test_scan_files(
        #[case] md_files: Vec<&str>,
        #[case] other_files: Vec<&str>,
        #[case] expected_count: usize,
    ) -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // .mdファイルを作成
        for file_path in md_files.iter() {
            let full_path = base_path.join(file_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full_path, "# Test Content")?;
        }

        // その他のファイルを作成
        for file_path in other_files.iter() {
            fs::write(base_path.join(file_path), "Other content")?;
        }

        let files = scan_obsidian_files(base_path)?;
        assert_eq!(files.len(), expected_count);

        Ok(())
    }

    #[rstest]
    #[case("lowercase.md", "# Lowercase")]
    #[case("uppercase.MD", "# Uppercase")]
    #[case("mixed.Md", "# Mixed")]
    fn test_scan_case_insensitive_extensions(
        #[case] filename: &str,
        #[case] content: &str,
    ) -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // 指定された拡張子でファイルを作成
        fs::write(base_path.join(filename), content)?;
        fs::write(base_path.join("not_markdown.txt"), "Not markdown")?;

        let files = scan_obsidian_files(base_path)?;

        assert_eq!(files.len(), 1); // 1つの.mdファイルが検出される
        assert_eq!(files[0].file_name().unwrap().to_string_lossy(), filename);

        Ok(())
    }
}
