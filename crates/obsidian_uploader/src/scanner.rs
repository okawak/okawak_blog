use crate::error::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Scans the specified directory for Markdown files (.md) and returns their paths.
pub fn scan_obsidian_files(publish_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut md_files: Vec<PathBuf> = WalkBuilder::new(publish_dir.as_ref())
        .hidden(true) // Exclude hidden files
        .git_ignore(false) // do NOT respect .gitignore
        .build() // single threaded walk (for multiple threads, use `build_parallel`)
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (entry.file_type().is_some_and(|ft| ft.is_file())
                && path
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md")))
            .then(|| entry.into_path())
        })
        .collect();

    // for performance reasons, we use `sort_unstable` instead of `sort`
    md_files.sort_unstable();
    Ok(md_files)
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

        // create Markdown file
        for file_path in md_files.iter() {
            let full_path = base_path.join(file_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full_path, "# Test Content")?;
        }

        // create other files
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

        fs::write(base_path.join(filename), content)?;
        fs::write(base_path.join("not_markdown.txt"), "Not markdown")?;

        let files = scan_obsidian_files(base_path)?;

        assert_eq!(files.len(), 1); // should find one Markdown file
        assert_eq!(files[0].file_name().unwrap().to_string_lossy(), filename);

        Ok(())
    }
}
