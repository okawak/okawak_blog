use crate::types::ArticleSummary;
use leptos::prelude::*;
use reactive_stores::Store;
use stylance::import_style;
use thaw::*;

import_style!(home_style, "home.module.scss");

/// 全てのカテゴリの記事一覧を取得するサーバー関数（ダミー実装）
#[server]
pub async fn get_latest_articles() -> Result<Vec<ArticleSummary>, ServerFnError> {
    // 一旦空のリストを返す
    Ok(vec![])
}

#[derive(Store, Clone)]
pub struct ArticlesData {
    #[store(key: String = |article: &ArticleSummary| article.id.to_string())]
    rows: Vec<ArticleSummary>,
}

/// ホームページコンポーネント
#[component]
pub fn HomePage() -> impl IntoView {
    // 最新記事一覧を取得
    let latest_articles = Resource::<Result<Vec<ArticleSummary>, String>>::new(
        || (),
        move |_| {
            async move {
                // SSR/CSR 両方で get_latest_articles を呼ぶ
                get_latest_articles().await.map_err(|e| e.to_string())
            }
        },
    );

    let articles_store = Store::new(ArticlesData { rows: vec![] });
    Effect::new(move |_| {
        if let Some(Ok(articles)) = latest_articles.get() {
            let rows = articles_store.rows();
            *rows.write() = articles.clone();
        }
    });

    view! {
        <div class=home_style::home_page>
            <section class=home_style::profile_section>
                <h1>{"ホーム"}</h1>
                <div class=home_style::profile_text>
                    <p>{"気になったことをメモしておくブログです。"}</p>
                </div>

            // thaw-ui + stylance統合デモ
            // <div class=home_style::thaw_ui_test_section>
            // <h3>{"Leptos 0.8 + thaw-ui + stylance 統合デモ"}</h3>

            // <div class=home_style::component_group>
            // <h4>{"テーマ統合ボタン"}</h4>
            // <Space vertical=false>
            // <Button>{"プライマリボタン"}</Button>
            // <Button>{"標準ボタン"}</Button>
            // <Button>{"アクションボタン"}</Button>
            // </Space>
            // </div>

            // <div class=home_style::component_group>
            // <h4>{"レスポンシブレイアウト"}</h4>
            // <Space vertical=true>
            // <div class=home_style::responsive_demo>
            // <div>{"モバイルファーストレスポンシブデザイン"}</div>
            // <div>{"CSS Custom Properties テーマシステム"}</div>
            // <div>{"stylance + thaw-ui ハイブリッド統合"}</div>
            // </div>
            // </Space>
            // </div>

            // <div class=home_style::component_group>
            // <h4>{"ハイブリッドカードレイアウト"}</h4>
            // <div class=home_style::hybrid_layout>
            // <div class=home_style::stylance_card>
            // <h5>{"stylance カスタムスタイル"}</h5>
            // <p>{"既存のCSS資産を活用しつつ、thaw-uiコンポーネントを統合"}</p>
            // <Space>
            // <Button>{"アクション1"}</Button>
            // <Button>{"アクション2"}</Button>
            // </Space>
            // </div>
            // <div class=home_style::stylance_card>
            // <h5>{"テーマ統一"}</h5>
            // <p>{"CSS Custom Propertiesでstylanceとthaw-uiのテーマを統一"}</p>
            // <Button>{"詳細を見る"}</Button>
            // </div>
            // <div class=home_style::stylance_card>
            // <h5>{"パフォーマンス最適化"}</h5>
            // <p>{"Leptos 0.8の新機能とthaw-uiによる高速レンダリング"}</p>
            // <Button>{"もっと見る"}</Button>
            // </div>
            // </div>
            // </div>

            // <div class=home_style::component_group>
            // <h4>{"アクセシビリティ対応"}</h4>
            // <div class=home_style::accessibility_demo>
            // <Space vertical=true>
            // <div class=home_style::demo_info>
            // {"キーボードナビゲーション、ARIA属性、カラーコントラスト対応"}
            // </div>
            // <Space>
            // <Button class="focus-visible">{"フォーカス表示"}</Button>
            // <Button>{"ARIA対応"}</Button>
            // </Space>
            // </Space>
            // </div>
            // </div>
            // </div>
            </section>

            <section class=home_style::latest_articles>
                <h2>{"最近の記事"}</h2>
                <Suspense fallback=|| {
                    view! { <div class=home_style::loading>"記事を読み込み中..."</div> }
                }>
                    <ErrorBoundary fallback=|error| {
                        view! {
                            <div class=home_style::error>
                                "記事の読み込みに失敗しました: "
                                {format!("{error:?}")}
                            </div>
                        }
                    }>
                        <Show
                            when=move || {
                                matches!(
                                    latest_articles.get(),
                                    Some(Ok(articles))
                                    if !articles.is_empty()
                                )
                            }
                            fallback=|| {
                                view! {
                                    <div class=home_style::no_articles>
                                        "記事がありません"
                                    </div>
                                }
                            }
                        >
                            <div class=home_style::no_articles>"記事がありません"</div>
                        </Show>
                    </ErrorBoundary>
                </Suspense>
            </section>
        </div>
    }
}

// WASM（hydrate）用 スタブ：型だけ合わせておく
#[cfg(not(feature = "ssr"))]
#[allow(dead_code)]
async fn fetch_latest_articles() -> Result<Vec<ArticleSummary>, String> {
    // クライアントナビゲーション時に呼び出されても型エラーにならないよう、
    // 空リスト or 適当なエラーを返す
    Ok(vec![])
}
