use crate::components::sidebar::Sidebar;
use leptos::prelude::*;
use leptos_meta::{Meta, Title}; // Titleも含めてleptos_metaからインポート

/// 404 Not Found ページコンポーネント
/// URLが存在しない場合に表示されるページ
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <>
            // メタ情報を直接指定（参照ではなく直接コンポーネントを使用）
            <Title text="404 - ページが見つかりません | ぶくせんの探窟メモ" />
            <Meta
                name="description"
                content="お探しのページが見つかりませんでした。"
            />
            <Meta name="robots" content="noindex" />

            <div class="not-found-page">
                <div class="main-content">
                    <div class="not-found-container">
                        <div class="not-found-header">
                            <h1>404</h1>
                            <h2>ページが見つかりません</h2>
                        </div>

                        <div class="not-found-message">
                            <p>
                                お探しのページが見つかりませんでした. URLが正しいか確認してください.
                            </p>
                            <p>
                                または, 以下のリンクからホームページに戻ることができます.
                            </p>
                        </div>

                        <div class="not-found-actions">
                            <a href="/" class="home-link">
                                <i class="fas fa-home"></i>
                                ホームに戻る
                            </a>
                        </div>

                        <div class="not-found-illustration">
                            <img
                                src="/assets/images/not_found.svg"
                                alt="ページが見つからないイラスト"
                            />
                        </div>
                    </div>
                </div>

                <Sidebar />
            </div>
        </>
    }
}
