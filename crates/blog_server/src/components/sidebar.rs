use leptos::prelude::*;

/// サイドバーコンポーネント
///
/// 人気記事、カテゴリー、タグなどの補助的なナビゲーション要素を表示します
#[component]
pub fn Sidebar(category: &'static str) -> impl IntoView {
    view! {
        <aside class="sidebar">
            <div class="sidebar-section">
                <h3 class="sidebar-title">カテゴリー</h3>
            // ...（カテゴリーリストの部分は変更なし）...
            </div>
        </aside>
    }
}
