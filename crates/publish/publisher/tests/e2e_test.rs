mod support;

use indoc::indoc;
use publisher::{Config, run_main, slug};
use std::{fs, path::Path};
use support::collect_html_files;
use tempfile::TempDir;

/// End-to-end test that simulates a realistic Obsidian vault.
#[tokio::test]
async fn test_end_to_end_obsidian_processing() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // Create a realistic Obsidian directory structure.
    fs::create_dir_all(&obsidian_dir).unwrap();
    fs::create_dir_all(obsidian_dir.join("tech")).unwrap();
    fs::create_dir_all(obsidian_dir.join("blog")).unwrap();

    // Sample technical article.
    let tech_article = indoc! {r#"
        ---
        title: "Rustでのパフォーマンス最適化"
        tags: ["rust", "performance", "optimization"]
        summary: "Rustアプリケーションのパフォーマンス最適化手法について詳しく解説します"
        priority: 1
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T15:30:00+09:00"
        is_completed: true
        category: "tech"
        ---

        # Rustでのパフォーマンス最適化

        この記事では、[[基本的なRust概念]]について理解している前提で、パフォーマンス最適化の技法を説明します。

        ## ベンチマークの重要性

        最適化の前に**計測**が重要です：

        ```rust
        use criterion::{black_box, criterion_group, criterion_main, Criterion};

        fn fibonacci(n: u64) -> u64 {
            match n {
                0 => 1,
                1 => 1,
                n => fibonacci(n-1) + fibonacci(n-2),
            }
        }

        fn benchmark_fibonacci(c: &mut Criterion) {
            c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
        }

        criterion_group!(benches, benchmark_fibonacci);
        criterion_main!(benches);
        ```

        ## メモリ最適化

        - Vector の事前確保
        - String vs &str の選択
        - Box, Rc, Arc の適切な使用

        詳細は[[メモリ管理のベストプラクティス]]を参照してください。

        ### 参考リンク

        <div class="bookmark">
          <a href="https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html">Rust Book - Ownership</a>
        </div>
    "#};

    let tech_file = obsidian_dir.join("tech").join("rust-performance.md");
    fs::write(&tech_file, tech_article).unwrap();

    // Foundational concepts article.
    let basic_concepts = indoc! {r#"
        ---
        title: "基本的なRust概念"
        tags: ["rust", "basics", "beginner"]
        summary: "Rustプログラミング言語の基本概念を学習者向けに解説"
        priority: 2
        created: "2025-01-10T09:00:00+09:00"
        updated: "2025-01-12T14:00:00+09:00"
        is_completed: true
        category: "tech"
        ---

        # 基本的なRust概念

        Rustの**所有権システム**について学びましょう。

        ## 所有権の基本ルール

        1. 各値は*所有者*を持つ
        2. 同時に存在できる所有者は1つだけ
        3. 所有者がスコープを外れると値は破棄される

        ### コード例

        ```rust
        fn main() {
            let s1 = String::from("hello");
            let s2 = s1; // s1はもう使えない

            println!("{}", s2); // OK
            // println!("{}", s1); // Compile error
        }
        ```

        次は[[Rustでのパフォーマンス最適化]]について学んでみましょう。
    "#};

    let basic_file = obsidian_dir.join("basic-rust-concepts.md");
    fs::write(&basic_file, basic_concepts).unwrap();

    // Draft blog article.
    let blog_draft = indoc! {r#"
        ---
        title: "開発日記: ブログシステムを作ってみた"
        tags: ["blog", "development", "diary"]
        summary: "個人ブログシステムの開発体験記"
        priority: 3
        created: "2025-01-20T20:00:00+09:00"
        updated: "2025-01-20T20:30:00+09:00"
        is_completed: false
        category: "blog"
        ---

        # 開発日記: ブログシステムを作ってみた

        まだ作成中です...
    "#};

    let blog_file = obsidian_dir.join("blog").join("development-diary.md");
    fs::write(&blog_file, blog_draft).unwrap();

    // Memory-management best-practices article.
    let memory_practices = indoc! {r#"
        ---
        title: "メモリ管理のベストプラクティス"
        tags: ["rust", "memory", "best-practices"]
        summary: "Rustにおけるメモリ管理の効率的な手法"
        priority: 1
        created: "2025-01-18T11:00:00+09:00"
        updated: "2025-01-19T16:45:00+09:00"
        is_completed: true
        category: "tech"
        ---

        # メモリ管理のベストプラクティス

        効率的なメモリ使用のための**実践的なテクニック**を紹介します。

        ## Vector の事前確保

        ```rust
        // Bad example
        let mut vec = Vec::new();
        for i in 0..1000 {
            vec.push(i);
        }

        // Good example
        let mut vec = Vec::with_capacity(1000);
        for i in 0..1000 {
            vec.push(i);
        }
        ```

        ## String vs &str

        - `&str`: 文字列スライス（不変）
        - `String`: 所有された文字列（可変）

        詳細な所有権の概念は[[基本的なRust概念]]を参照。

        ### 数学的な例

        メモリ効率の計算式：

        $$
        \text{効率} = \frac{\text{使用メモリ}}{\text{確保メモリ}} \times 100\%
        $$

        インライン数式の例：$O(n)$ の計算量。
    "#};

    let memory_file = obsidian_dir.join("tech").join("memory-best-practices.md");
    fs::write(&memory_file, memory_practices).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // Run the main processing flow.
    let result = run_main(&config).await;
    assert!(result.is_ok(), "run_main should succeed");

    // Validate the output directory.
    assert!(output_dir.exists(), "Output directory should exist");

    // Validate the generated slug-based HTML files.
    let tech_slug = slug::generate_slug(
        "Rustでのパフォーマンス最適化",
        Path::new("tech/rust-performance.md"),
        "2025-01-15T10:00:00+09:00",
    )
    .unwrap();
    let basic_slug = slug::generate_slug(
        "基本的なRust概念",
        Path::new("basic-rust-concepts.md"),
        "2025-01-10T09:00:00+09:00",
    )
    .unwrap();
    let memory_slug = slug::generate_slug(
        "メモリ管理のベストプラクティス",
        Path::new("tech/memory-best-practices.md"),
        "2025-01-18T11:00:00+09:00",
    )
    .unwrap();

    let site_root = output_dir.join("site");
    let articles_dir = site_root.join("articles");
    let _tech_html = articles_dir.join("tech").join(format!("{tech_slug}.html"));
    let _basic_html = articles_dir.join("tech").join(format!("{basic_slug}.html"));
    let _memory_html = articles_dir
        .join("tech")
        .join(format!("{memory_slug}.html"));

    // Draft article slug. No HTML should be generated because `is_completed` is false.
    let blog_slug = slug::generate_slug(
        "開発日記: ブログシステムを作ってみた",
        Path::new("blog/development-diary.md"),
        "2025-01-20T20:00:00+09:00",
    )
    .unwrap();
    let blog_html = articles_dir.join("blog").join(format!("{blog_slug}.html"));

    // Check that completed articles produced HTML by counting files.
    let html_count = collect_html_files(&articles_dir).len();

    // Expected files: two technical articles plus one foundational article.
    assert_eq!(html_count, 3, "Should generate 3 published articles");

    // Ensure the draft article was not generated.
    assert!(
        !blog_html.exists(),
        "Draft blog HTML should not be generated"
    );

    // Inspect generated HTML and look for specific content in the articles directory.
    let mut performance_file_found = false;
    let mut memory_file_found = false;

    let files = collect_html_files(&articles_dir);
    if !files.is_empty() {
        println!("Found HTML files in site/articles: {:?}", files);

        for file in files {
            if let Ok(content) = fs::read_to_string(&file) {
                println!("Checking file: {:?}", file);

                // Safe string slice.
                let preview_len = content
                    .char_indices()
                    .nth(100)
                    .map(|(i, _)| i)
                    .unwrap_or(content.len());
                println!(
                    "File content preview (first 100 chars): {}",
                    &content[..preview_len]
                );

                if content.contains("Rustでのパフォーマンス最適化") && content.contains("fibonacci")
                {
                    performance_file_found = true;
                    println!("Found performance optimization file");
                }
                if content.contains("メモリ管理のベストプラクティス") {
                    memory_file_found = true;
                    println!("Found memory management file");
                }
            }
        }
    } else {
        println!("Could not read site/articles directory");
    }

    assert!(
        performance_file_found,
        "Performance optimization article should be present in site/articles"
    );
    assert!(
        memory_file_found,
        "Memory management article should be present in site/articles"
    );

    // Verify KaTeX math rendering.
    let mut math_processing_verified = false;

    for path in collect_html_files(&articles_dir) {
        if let Ok(content) = fs::read_to_string(path)
            && content.contains("<span class=\"okawak-katex-display\">")
            && content.contains("<span class=\"okawak-katex-inline\">")
        {
            math_processing_verified = true;
        }
    }

    assert!(
        math_processing_verified,
        "KaTeX math processing should work in tech files"
    );

    let article_index = fs::read_to_string(articles_dir.join("index.json")).unwrap();
    assert!(article_index.contains(&tech_slug));
    assert!(article_index.contains(&basic_slug));
    assert!(article_index.contains(&memory_slug));

    let tech_category_index =
        fs::read_to_string(site_root.join("categories").join("tech").join("index.json")).unwrap();
    assert!(tech_category_index.contains("\"category\": \"tech\""));

    let site_metadata = fs::read_to_string(site_root.join("metadata").join("site.json")).unwrap();
    assert!(site_metadata.contains("\"total_articles\": 3"));

    println!(
        "✅ End-to-end test finished: generated {} HTML files",
        collect_html_files(&articles_dir).len()
    );
}

/// Performance test for processing a large number of files.
#[tokio::test]
async fn test_large_volume_processing() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    fs::create_dir_all(&obsidian_dir).unwrap();

    // Generate 100 test files.
    for i in 0..100 {
        let content = format!(
            indoc! {r#"
            ---
            title: "Test Article {}"
            tags: ["test", "performance"]
            summary: "Performance test article number {}"
            priority: {}
            created: "2025-01-{:02}T{:02}:00:00+09:00"
            updated: "2025-01-{:02}T{:02}:30:00+09:00"
            is_completed: true
            category: "tech"
            ---

            # Test Article {}

            This is test article number **{}**.

            ## Content Section

            Lorem ipsum dolor sit amet, consectetur adipiscing elit.

            ### Code Example

            ```rust
            fn test_function_{}() {{
                println!("Test {}", {});
            }}
            ```

            Link to [[Test Article {}]] if it exists.
        "#},
            i,
            i,
            (i % 3) + 1,
            (i % 28) + 1,
            (i % 24),
            (i % 28) + 1,
            (i % 24),
            i,
            i,
            i,
            i,
            i,
            (i + 1) % 100
        );

        let file_path = obsidian_dir.join(format!("test-article-{i:03}.md"));
        fs::write(&file_path, content).unwrap();
    }

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // Measure processing time.
    let start = std::time::Instant::now();
    let result = run_main(&config).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Large volume processing should succeed");

    // Verify the number of generated files.
    let generated_count = collect_html_files(&output_dir.join("site").join("articles")).len();

    assert_eq!(generated_count, 100, "Should generate 100 HTML files");

    // Assert the performance target: 100 files within 5 seconds.
    assert!(
        duration.as_secs() < 5,
        "Should process 100 files in under 5 seconds, took {duration:?}"
    );

    println!("✅ Performance test finished: processed {generated_count} files in {duration:.2?}");
}

/// Error-handling test for continuing after partial failures.
#[tokio::test]
async fn test_partial_failure_handling() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    fs::create_dir_all(&obsidian_dir).unwrap();

    // Valid file.
    let valid_file = indoc! {r#"
        ---
        title: "Valid Article"
        tags: ["test"]
        summary: "This is a valid article"
        priority: 1
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T10:30:00+09:00"
        is_completed: true
        category: "tech"
        ---

        # Valid Article

        This article should be processed successfully.
    "#};

    // File with malformed YAML front matter.
    let invalid_yaml = indoc! {r#"
        ---
        title: "Invalid YAML"
        tags: [invalid yaml structure
        summary: "This has broken YAML"
        priority: not_a_number
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T10:30:00+09:00"
        is_completed: true
        category: "tech"
        ---

        # Invalid YAML Article

        This article has broken frontmatter.
    "#};

    // Incomplete file that should be skipped.
    let incomplete_file = indoc! {r#"
        ---
        title: "Incomplete Article"
        tags: ["test"]
        summary: "This is incomplete"
        priority: 1
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T10:30:00+09:00"
        is_completed: false
        category: "tech"
        ---

        # Incomplete Article

        This should be skipped.
    "#};

    fs::write(obsidian_dir.join("valid.md"), valid_file).unwrap();
    fs::write(obsidian_dir.join("invalid.md"), invalid_yaml).unwrap();
    fs::write(obsidian_dir.join("incomplete.md"), incomplete_file).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // Processing should continue even when some files fail.
    let result = run_main(&config).await;
    assert!(
        result.is_ok(),
        "Should continue processing despite partial failures"
    );

    // The valid file should still be processed.
    let valid_slug = slug::generate_slug(
        "Valid Article",
        Path::new("valid.md"),
        "2025-01-15T10:00:00+09:00",
    )
    .unwrap();
    let valid_html = output_dir
        .join("site")
        .join("articles")
        .join("tech")
        .join(format!("{valid_slug}.html"));
    assert!(valid_html.exists(), "Valid file should be processed");

    // The malformed file should not be processed.
    let invalid_slug = slug::generate_slug(
        "Malformed Article", // Placeholder title used for invalid.md.
        Path::new("invalid.md"),
        "2025-01-15T10:00:00+09:00",
    )
    .unwrap();
    let invalid_html = output_dir
        .join("site")
        .join("articles")
        .join("tech")
        .join(format!("{invalid_slug}.html"));
    assert!(
        !invalid_html.exists(),
        "Invalid file should not be processed"
    );

    // The incomplete file should not be processed.
    let incomplete_html = output_dir
        .join("site")
        .join("articles")
        .join("tech")
        .join("incomplete.html");
    assert!(
        !incomplete_html.exists(),
        "Incomplete file should not be processed"
    );

    println!("✅ Error-handling test finished: valid files still complete after partial failures");
}
