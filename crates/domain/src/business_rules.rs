//! Business Rules - 純粋なビジネスルール関数
//!
//! I/Oなし、依存なしの純粋関数でビジネスロジックを実装

use crate::{Article, ArticleSummary, Category, DomainError, Result, Slug, Title};
use std::collections::HashMap;

// =============================================================================
// S3 Path Processing Rules - S3パス処理ルール（純粋関数）
// =============================================================================

/// S3パスからSlugを抽出する純粋関数
/// 
/// # 例
/// - "tech/a1b2c3d4e5f6.html" -> "a1b2c3d4e5f6"
/// - "blog/f6e5d4c3b2a1.html" -> "f6e5d4c3b2a1"
pub fn extract_slug_from_s3_path(s3_path: &str) -> Result<Slug> {
    // 空文字列チェック
    if s3_path.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // 先頭のスラッシュは無効なパスとして扱う
    if s3_path.starts_with('/') {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // 二重スラッシュも無効なパスとして扱う
    if s3_path.contains("//") {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // "category/slug.html" 形式から "slug.html" を抽出
    let file_name = s3_path
        .split('/')
        .last()
        .ok_or_else(|| DomainError::InvalidPath {
            path: s3_path.to_string(),
        })?;
    
    // ".html" 拡張子を除去して slug を取得
    let slug_str = file_name
        .strip_suffix(".html")
        .ok_or_else(|| DomainError::InvalidPath {
            path: s3_path.to_string(),
        })?;
    
    // 空のslugチェック
    if slug_str.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // Slugエンティティを作成
    Slug::new(slug_str.to_string())
}

/// S3パスからCategoryを抽出する純粋関数
/// 
/// # 例
/// - "tech/a1b2c3d4e5f6.html" -> Category::Tech
/// - "daily/9876543210ab.html" -> Category::Daily
pub fn extract_category_from_s3_path(s3_path: &str) -> Result<Category> {
    // 空文字列チェック
    if s3_path.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // 先頭のスラッシュは無効なパスとして扱う
    if s3_path.starts_with('/') {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // "category/file.html" 形式でない場合は無効（スラッシュが含まれている必要がある）
    if !s3_path.contains('/') {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // パスから最初のセグメント（カテゴリ）を取得
    let category_str = s3_path
        .split('/')
        .next()
        .ok_or_else(|| DomainError::InvalidPath {
            path: s3_path.to_string(),
        })?;
    
    // カテゴリ名が空でないことを確認
    if category_str.is_empty() {
        return Err(DomainError::InvalidPath {
            path: s3_path.to_string(),
        });
    }
    
    // 文字列をCategoryに変換（FromStrトレイト使用）
    category_str.parse().map_err(|_| DomainError::InvalidPath {
        path: s3_path.to_string(),
    })
}

// =============================================================================
// Slug Generation Rules - スラッグ生成ルール
// =============================================================================

/// タイトルからスラッグを生成（純粋関数）
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

/// スラッグのユニーク性を確保（純粋関数）
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

    // 最悪の場合のフォールバック（タイムスタンプベース）
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}-{}", base, timestamp)
}

// =============================================================================
// Article Statistics Rules - 記事統計ルール
// =============================================================================

/// 記事統計情報
#[derive(Debug, Clone)]
pub struct ArticleStats {
    pub total_articles: usize,
    pub published_articles: usize,
    pub categories: HashMap<Category, usize>,
    pub popular_tags: Vec<(String, usize)>,
}

/// 記事一覧から統計を計算（純粋関数）
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
    popular_tags.truncate(10); // トップ10タグのみ

    ArticleStats {
        total_articles,
        published_articles,
        categories,
        popular_tags,
    }
}

// =============================================================================
// Content Processing Rules - コンテンツ処理ルール
// =============================================================================

/// コンテンツからサマリーを生成（純粋関数）
pub fn generate_summary(content: &str, max_length: usize) -> String {
    let cleaned = strip_markdown(content);

    if cleaned.len() <= max_length {
        return cleaned;
    }

    // 適切な位置で切断（句読点を考慮）
    let truncated = &cleaned[..max_length];
    if let Some(last_period) = truncated.rfind('。') {
        format!("{}。", &truncated[..last_period])
    } else if let Some(last_comma) = truncated.rfind('、') {
        format!("{}...", &truncated[..last_comma])
    } else {
        format!("{}...", truncated)
    }
}

/// Markdownから基本的な記号を除去（純粋関数）
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

/// 読了時間を推定（純粋関数、日本語対応）
pub fn estimate_reading_time(content: &str) -> usize {
    let char_count = content.chars().count();
    // 日本語: 1分間に約400-600文字読める（平均500文字で計算）
    let minutes = (char_count as f64 / 500.0).ceil() as usize;
    std::cmp::max(1, minutes) // 最低1分
}

// =============================================================================
// Validation Rules - バリデーションルール
// =============================================================================

/// 記事の公開前バリデーション（純粋関数）
pub fn validate_for_publishing(article: &Article) -> Result<()> {
    // タイトルが空でないことを確認
    if article.title.as_str().trim().is_empty() {
        return Err(DomainError::validation("タイトルが空です"));
    }

    // コンテンツが最低限の長さを持つことを確認
    if article.content.trim().len() < 10 {
        return Err(DomainError::validation(
            "コンテンツが短すぎます（最低10文字必要）",
        ));
    }

    // スラッグが適切な形式であることを確認
    if article.slug.as_str().is_empty() {
        return Err(DomainError::validation("スラッグが空です"));
    }

    Ok(())
}

/// カテゴリ変更のバリデーション（純粋関数）
pub fn validate_category_change(from: Category, to: Category, _article: &Article) -> Result<()> {
    // 同じカテゴリへの変更は不要
    if from == to {
        return Err(DomainError::validation(
            "同じカテゴリに変更することはできません",
        ));
    }

    // 将来的にビジネスルールを追加可能
    // 例: 特定のカテゴリからの変更制限など

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

    // 新規追加: S3パス解析のテスト (TDD Red Phase)
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
    #[case("tech/slug")]  // .htmlなし
    #[case("/tech/slug.html")]  // 先頭スラッシュ
    #[case("tech//slug.html")]  // 二重スラッシュ
    fn test_extract_slug_from_s3_path_failure(#[case] invalid_path: &str) {
        let result = extract_slug_from_s3_path(invalid_path);
        assert!(result.is_err(), "Expected error for path: {}", invalid_path);
    }

    #[rstest]
    #[case("tech/a1b2c3d4e5f6.html", Category::Tech)]
    #[case("daily/f6e5d4c3b2a1.html", Category::Daily)]
    #[case("statistics/abcd1234567e.html", Category::Statistics)]
    #[case("physics/ef123456789a.html", Category::Physics)]
    fn test_extract_category_from_s3_path_success(#[case] s3_path: &str, #[case] expected_category: Category) {
        let result = extract_category_from_s3_path(s3_path).unwrap();
        assert_eq!(result, expected_category);
    }

    #[rstest]
    #[case("")]
    #[case("invalid/file.html")]
    #[case("unknown_category/slug.html")]
    #[case("/tech/slug.html")]  // 先頭スラッシュ
    #[case("tech")]  // ファイル名なし
    fn test_extract_category_from_s3_path_failure(#[case] invalid_path: &str) {
        let result = extract_category_from_s3_path(invalid_path);
        assert!(result.is_err(), "Expected error for path: {}", invalid_path);
    }
}
