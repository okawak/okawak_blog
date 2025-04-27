use leptos::prelude::*;

/// サイドバーコンポーネント
///
/// 人気記事、カテゴリー、タグなどの補助的なナビゲーション要素を表示します
#[component]
pub fn Sidebar(category: &'static str, class: &'static str) -> impl IntoView {
    view! {
        <aside class=class>
            <div class="sidebar-section">
                <h3 class="sidebar-title">{category}</h3>
            </div>
        </aside>
    }
}
