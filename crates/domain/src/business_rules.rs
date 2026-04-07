//! Pure business-rule functions.
//!
//! Business logic implemented as pure functions without I/O or external dependencies.

use crate::{Article, ArticleSummary, Category, DomainError, Result, Slug, Title};
use std::collections::HashMap;

// =============================================================================
// Pure rules for processing S3 paths.
// =============================================================================

/// Extracts a slug from an S3 path.
///
/// # Examples
/// - "tech/a1b2c3d4e5f6.html" -> "a1b2c3d4e5f6"
/// - "blog/f6e5d4c3b2a1.html" -> "f6e5d4c3b2a1"
pub fn extract_slug_from_s3_path(s3_path: &str) -> Result<Slug> {
    // Reject empty paths.
    if s3_path.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Treat leading slashes as invalid paths.
    if s3_path.starts_with('/') {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Treat double slashes as invalid paths.
    if s3_path.contains("//") {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Extract "slug.html" from the "category/slug.html" shape.
    let file_name = s3_path
        .split('/')
        .next_back()
        .ok_or_else(|| DomainError::InvalidPath {
            path: s3_path.to_string(),
        })?;

    // Remove the ".html" extension to obtain the slug.
    let slug_str = file_name
        .strip_suffix(".html")
        .ok_or_else(|| DomainError::InvalidPath {
            path: s3_path.to_string(),
        })?;

    // Reject empty slugs.
    if slug_str.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Build the Slug value object.
    Slug::new(slug_str.to_string())
}

/// Extracts a category from an S3 path.
///
/// # Examples
/// - "tech/a1b2c3d4e5f6.html" -> Category::Tech
/// - "daily/9876543210ab.html" -> Category::Daily
pub fn extract_category_from_s3_path(s3_path: &str) -> Result<Category> {
    // Reject empty paths.
    if s3_path.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Treat leading slashes as invalid paths.
    if s3_path.starts_with('/') {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Reject paths that do not match the "category/file.html" shape.
    if !s3_path.contains('/') {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Take the first path segment as the category.
    let category_str = s3_path
        .split('/')
        .next()
        .ok_or_else(|| DomainError::InvalidPath {
            path: s3_path.to_string(),
        })?;

    // Reject empty category names.
    if category_str.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }

    // Parse the string into Category via FromStr.
    category_str.parse().map_err(|_| DomainError::InvalidPath {
        path: s3_path.to_string(),
    })
}

// =============================================================================
// Rules for slug generation.
// =============================================================================

/// Generates a slug from a title.
pub fn generate_slug_from_title(title: &Title) -> Result<Slug> {
    let title_str = title.as_str();
    let slug_str = title_str
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .trim_matches('_')
        .to_string();

    Slug::new(slug_str)
}

/// Ensures slug uniqueness.
pub fn ensure_unique_slug(base_slug: &Slug, existing_slugs: &[String]) -> String {
    let base = base_slug.as_str();

    if !existing_slugs.contains(&base.to_string()) {
        return base.to_string();
    }

    for i in 1..=999 {
        let candidate = format!("{}-{}", base, i);
        if !existing_slugs.contains(&candidate) {
            return candidate;
        }
    }

    // Timestamp-based fallback for the worst case.
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}-{}", base, timestamp)
}

// =============================================================================
// Rules for article statistics.
// =============================================================================

/// Aggregate article statistics.
#[derive(Debug, Clone)]
pub struct ArticleStats {
    pub total_articles: usize,
    pub published_articles: usize,
    pub categories: HashMap<Category, usize>,
    pub popular_tags: Vec<(String, usize)>,
}

/// Calculates statistics from a list of article summaries.
pub fn calculate_article_stats(articles: &[ArticleSummary]) -> ArticleStats {
    let total_articles = articles.len();
    let published_articles = articles.iter().filter(|a| a.is_published).count();

    let mut categories = HashMap::new();
    let mut tag_counts = HashMap::new();

    for article in articles {
        if article.is_published {
            *categories.entry(article.category).or_insert(0) += 1;

            for tag in &article.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut popular_tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
    popular_tags.sort_by(|a, b| b.1.cmp(&a.1));
    popular_tags.truncate(10); // Keep only the top 10 tags.

    ArticleStats {
        total_articles,
        published_articles,
        categories,
        popular_tags,
    }
}

// =============================================================================
// Rules for content processing.
// =============================================================================

/// Generates a summary from content.
pub fn generate_summary(content: &str, max_length: usize) -> String {
    let cleaned = strip_markdown(content);

    if cleaned.len() <= max_length {
        return cleaned;
    }

    // Truncate at a reasonable boundary with punctuation awareness.
    let truncated = &cleaned[..max_length];
    if let Some(last_period) = truncated.rfind('。') {
        format!("{}。", &truncated[..last_period])
    } else if let Some(last_comma) = truncated.rfind('、') {
        format!("{}...", &truncated[..last_comma])
    } else {
        format!("{}...", truncated)
    }
}

/// Removes basic Markdown markers.
pub fn strip_markdown(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches('#')
                .trim_start_matches('*')
                .trim_start_matches('-')
                .trim()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Estimates reading time for Japanese content.
pub fn estimate_reading_time(content: &str) -> usize {
    let char_count = content.chars().count();
    // Japanese readers typically read roughly 400-600 characters per minute.
    let minutes = (char_count as f64 / 500.0).ceil() as usize;
    std::cmp::max(1, minutes) // Always return at least one minute.
}

// =============================================================================
// Validation rules.
// =============================================================================

/// Validates an article before publishing.
pub fn validate_for_publishing(article: &Article) -> Result<()> {
    // Require a non-empty title.
    if article.title.as_str().trim().is_empty() {
        return Err(DomainError::validation("タイトルが空です"));
    }

    // Require the content to meet a minimum length.
    if article.content.trim().len() < 10 {
        return Err(DomainError::validation(
            "コンテンツが短すぎます（最低10文字必要）",
        ));
    }

    // Require a non-empty slug.
    if article.slug.as_str().is_empty() {
        return Err(DomainError::validation("スラッグが空です"));
    }

    Ok(())
}

/// Validates a category change.
pub fn validate_category_change(from: Category, to: Category, _article: &Article) -> Result<()> {
    // Reject no-op category changes.
    if from == to {
        return Err(DomainError::validation(
            "同じカテゴリに変更することはできません",
        ));
    }

    // Additional business rules can be added here in the future.
    // Example: restricting transitions from specific categories.

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Category, Title};
    use rstest::*;

    #[test]
    fn test_generate_slug_from_title() {
        let title = Title::new("Hello World Test".to_string()).unwrap();
        let slug = generate_slug_from_title(&title).unwrap();
        assert_eq!(slug.as_str(), "hello-world-test");
    }

    #[test]
    fn test_ensure_unique_slug() {
        let slug = Slug::new("test-slug".to_string()).unwrap();
        let existing = vec!["test-slug".to_string(), "test-slug-1".to_string()];
        let unique = ensure_unique_slug(&slug, &existing);
        assert_eq!(unique, "test-slug-2");
    }

    #[test]
    fn test_strip_markdown() {
        let content = "# タイトル\n\n* リスト項目\n- 別のリスト\n\n通常のテキスト";
        let stripped = strip_markdown(content);
        assert_eq!(stripped, "タイトル リスト項目 別のリスト 通常のテキスト");
    }

    // Added coverage for S3 path parsing during the TDD red phase.
    #[rstest]
    #[case("tech/a1b2c3d4e5f6.html", "a1b2c3d4e5f6")]
    #[case("blog/f6e5d4c3b2a1.html", "f6e5d4c3b2a1")]
    #[case("daily/9876543210ab.html", "9876543210ab")]
    #[case("statistics/abcd1234567e.html", "abcd1234567e")]
    #[case("physics/ef123456789a.html", "ef123456789a")]
    fn test_extract_slug_from_s3_path_success(#[case] s3_path: &str, #[case] expected_slug: &str) {
        let result = extract_slug_from_s3_path(s3_path).unwrap();
        assert_eq!(result.as_str(), expected_slug);
    }

    #[rstest]
    #[case("invalid-path")]
    #[case("tech/")]
    #[case("tech/file.txt")]
    #[case("")]
    #[case("tech/slug")] // Missing .html extension.
    #[case("/tech/slug.html")] // Leading slash.
    #[case("tech//slug.html")] // Double slash.
    fn test_extract_slug_from_s3_path_failure(#[case] invalid_path: &str) {
        let result = extract_slug_from_s3_path(invalid_path);
        assert!(result.is_err(), "Expected error for path: {}", invalid_path);
    }

    #[rstest]
    #[case("tech/a1b2c3d4e5f6.html", Category::Tech)]
    #[case("daily/f6e5d4c3b2a1.html", Category::Daily)]
    #[case("statistics/abcd1234567e.html", Category::Statistics)]
    #[case("physics/ef123456789a.html", Category::Physics)]
    fn test_extract_category_from_s3_path_success(
        #[case] s3_path: &str,
        #[case] expected_category: Category,
    ) {
        let result = extract_category_from_s3_path(s3_path).unwrap();
        assert_eq!(result, expected_category);
    }

    #[rstest]
    #[case("")]
    #[case("invalid/file.html")]
    #[case("unknown_category/slug.html")]
    #[case("/tech/slug.html")] // Leading slash.
    #[case("tech")] // Missing file name.
    fn test_extract_category_from_s3_path_failure(#[case] invalid_path: &str) {
        let result = extract_category_from_s3_path(invalid_path);
        assert!(result.is_err(), "Expected error for path: {}", invalid_path);
    }
}
