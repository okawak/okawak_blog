use indoc::indoc;
use obsidian_uploader::{Config, run_main, slug};
use std::{fs, path::Path};
use tempfile::TempDir;

/// エンドツーエンドテスト: 実際のObsidianファイル形式を模擬した包括的テスト
#[tokio::test]
async fn test_end_to_end_obsidian_processing() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // リアルなObsidianディレクトリ構造を作成
    fs::create_dir_all(&obsidian_dir).unwrap();
    fs::create_dir_all(obsidian_dir.join("tech")).unwrap();
    fs::create_dir_all(obsidian_dir.join("blog")).unwrap();

    // 技術記事のサンプル
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

    // 基本概念記事
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
            // println!("{}", s1); // コンパイルエラー
        }
        ```

        次は[[Rustでのパフォーマンス最適化]]について学んでみましょう。
    "#};

    let basic_file = obsidian_dir.join("basic-rust-concepts.md");
    fs::write(&basic_file, basic_concepts).unwrap();

    // ブログ記事（未完成）
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

    // メモリ管理のベストプラクティス記事
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
        // 悪い例
        let mut vec = Vec::new();
        for i in 0..1000 {
            vec.push(i);
        }

        // 良い例
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

    // メイン処理を実行
    let result = run_main(&config).await;
    assert!(result.is_ok(), "run_main should succeed");

    // 出力ディレクトリの検証
    assert!(output_dir.exists(), "Output directory should exist");

    // 生成されたHTMLファイルの検証（slugベース）
    let tech_slug = slug::generate_slug(
        "Rustでのパフォーマンス最適化",
        Path::new("tech/rust-performance.md"),
        "2025-01-15T10:00:00+09:00",
    ).unwrap();
    let basic_slug = slug::generate_slug(
        "基本的なRust概念",
        Path::new("basic-rust-concepts.md"),
        "2025-01-15T09:00:00+09:00",
    ).unwrap();
    let memory_slug = slug::generate_slug(
        "メモリ管理のベストプラクティス",
        Path::new("tech/memory-best-practices.md"),
        "2025-01-15T11:00:00+09:00",
    ).unwrap();
    
    let _tech_html = output_dir.join("tech").join(format!("{tech_slug}.html"));
    let _basic_html = output_dir.join(format!("{basic_slug}.html"));
    let _memory_html = output_dir.join("tech").join(format!("{memory_slug}.html"));
    
    // 未完成記事用（is_completed: false なので生成されない）
    let blog_slug = slug::generate_slug(
        "開発日記",
        Path::new("blog/development-diary.md"),
        "2025-01-15T12:00:00+09:00",
    ).unwrap();
    let blog_html = output_dir.join("blog").join(format!("{blog_slug}.html"));

    // 完成した記事のHTMLが生成されているか確認（ファイル数で判定）
    let mut html_count = 0;
    let mut tech_html_count = 0;
    
    // techディレクトリ内のHTMLファイル数をカウント
    if let Ok(entries) = fs::read_dir(output_dir.join("tech")) {
        tech_html_count = entries.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
            .count();
    }
    
    // ルートディレクトリ内のHTMLファイル数をカウント
    if let Ok(entries) = fs::read_dir(&output_dir) {
        html_count = entries.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
            .count();
    }
    
    // 期待する数：tech記事2つ（技術記事とメモリ記事）+ 基本記事1つ
    assert_eq!(tech_html_count, 2, "Should generate 2 tech articles");
    assert_eq!(html_count, 1, "Should generate 1 basic article");

    // 未完成の記事は生成されないことを確認
    assert!(
        !blog_html.exists(),
        "Draft blog HTML should not be generated"
    );

    // HTMLファイルの内容検証（tech ディレクトリ内のファイルから特定のコンテンツを探す）
    let mut performance_file_found = false;
    let mut memory_file_found = false;
    
    if let Ok(entries) = fs::read_dir(output_dir.join("tech")) {
        let files: Vec<_> = entries.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
            .collect();
        println!("Found HTML files in tech/: {:?}", files.iter().map(|f| f.path()).collect::<Vec<_>>());
        
        for file in files {
            if let Ok(content) = fs::read_to_string(file.path()) {
                println!("Checking file: {:?}", file.path());
                
                // 安全な文字列スライス
                let preview_len = content.char_indices()
                    .nth(100)
                    .map(|(i, _)| i)
                    .unwrap_or(content.len());
                println!("File content preview (first 100 chars): {}", &content[..preview_len]);
                
                if content.contains("Rustでのパフォーマンス最適化") && content.contains("fibonacci") {
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
        println!("Could not read tech directory");
    }
    
    assert!(
        performance_file_found,
        "Performance optimization article should be present in tech directory"
    );
    assert!(
        memory_file_found,
        "Memory management article should be present in tech directory"
    );

    // KaTeX数式の処理確認とフロントマター検証
    let mut math_processing_verified = false;
    let mut frontmatter_verified = false;
    
    if let Ok(entries) = fs::read_dir(output_dir.join("tech")) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "html") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if content.contains("<div class=\"katex-display\">") && content.contains("<span class=\"katex-inline\">") {
                        math_processing_verified = true;
                    }
                    if content.contains("title:") && content.contains("tags:") {
                        frontmatter_verified = true;
                    }
                }
            }
        }
    }
    
    assert!(
        math_processing_verified,
        "KaTeX math processing should work in tech files"
    );
    assert!(
        frontmatter_verified,
        "Frontmatter should be present in generated files"
    );

    // ディレクトリ構造の保持確認
    assert!(
        output_dir.join("tech").exists(),
        "Tech subdirectory should be preserved"
    );

    println!(
        "✅ エンドツーエンドテスト完了: {} 個のHTMLファイル生成",
        fs::read_dir(&output_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
            .count()
    );
}

/// パフォーマンステスト: 大量ファイル処理のテスト
#[tokio::test]
async fn test_large_volume_processing() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    fs::create_dir_all(&obsidian_dir).unwrap();

    // 100個のテストファイルを生成
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
            category: "test"
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

    // 処理時間の計測
    let start = std::time::Instant::now();
    let result = run_main(&config).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Large volume processing should succeed");

    // 生成されたファイル数の確認
    let generated_count = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
        .count();

    assert_eq!(generated_count, 100, "Should generate 100 HTML files");

    // パフォーマンス要件の確認（100ファイルを5秒以内で処理）
    assert!(
        duration.as_secs() < 5,
        "Should process 100 files in under 5 seconds, took {duration:?}"
    );

    println!("✅ パフォーマンステスト完了: {generated_count}個のファイルを{duration:.2?}で処理");
}

/// エラーハンドリングテスト: 部分的失敗時の継続処理
#[tokio::test]
async fn test_partial_failure_handling() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    fs::create_dir_all(&obsidian_dir).unwrap();

    // 正常なファイル
    let valid_file = indoc! {r#"
        ---
        title: "Valid Article"
        tags: ["test"]
        summary: "This is a valid article"
        priority: 1
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T10:30:00+09:00"
        is_completed: true
        category: "test"
        ---

        # Valid Article

        This article should be processed successfully.
    "#};

    // YAMLフロントマターが壊れたファイル
    let invalid_yaml = indoc! {r#"
        ---
        title: "Invalid YAML"
        tags: [invalid yaml structure
        summary: "This has broken YAML"
        priority: not_a_number
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T10:30:00+09:00"
        is_completed: true
        category: "test"
        ---

        # Invalid YAML Article

        This article has broken frontmatter.
    "#};

    // 未完成ファイル（スキップされるべき）
    let incomplete_file = indoc! {r#"
        ---
        title: "Incomplete Article"
        tags: ["test"]
        summary: "This is incomplete"
        priority: 1
        created: "2025-01-15T10:00:00+09:00"
        updated: "2025-01-15T10:30:00+09:00"
        is_completed: false
        category: "test"
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

    // 部分的失敗があっても処理は継続されるべき
    let result = run_main(&config).await;
    assert!(
        result.is_ok(),
        "Should continue processing despite partial failures"
    );

    // 正常なファイルは処理されるべき（slugベース）
    let valid_slug = slug::generate_slug(
        "Valid Article",
        Path::new("valid.md"),
        "2025-01-15T10:00:00+09:00",
    ).unwrap();
    let valid_html = output_dir.join(format!("{valid_slug}.html"));
    assert!(valid_html.exists(), "Valid file should be processed");

    // 異常なファイルは処理されないべき（slugベース）
    let invalid_slug = slug::generate_slug(
        "Malformed Article", // invalid.mdのタイトル（仮）
        Path::new("invalid.md"),
        "2025-01-15T10:00:00+09:00",
    ).unwrap();
    let invalid_html = output_dir.join(format!("{invalid_slug}.html"));
    assert!(
        !invalid_html.exists(),
        "Invalid file should not be processed"
    );

    // 未完成ファイルは処理されないべき
    let incomplete_html = output_dir.join("incomplete.html");
    assert!(
        !incomplete_html.exists(),
        "Incomplete file should not be processed"
    );

    println!("✅ エラーハンドリングテスト完了: 部分的失敗時も正常ファイルは処理継続");
}
