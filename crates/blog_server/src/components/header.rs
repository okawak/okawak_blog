use crate::components::NavigationItem;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// サイトヘッダーコンポーネント
#[component]
pub fn Header() -> impl IntoView {
    // 現在のパスを取得して、アクティブなナビゲーション項目を判断するために使用
    let location = use_location();
    let current_path = move || location.pathname.get();

    // クロージャを明示的な変数に格納し、型情報を提供
    let nav_items: Box<dyn Fn() -> Vec<NavigationItem> + Send + 'static> =
        Box::new(move || crate::components::get_main_nav_items(&current_path()));

    view! {
        <header class="site-header">
            <div class="header-container">
                <div class="logo">
                    <a href="/">
                        <h1>ぶくせんの探窟メモ</h1>
                    </a>
                </div>

                <nav class="main-nav">
                    <Navigation items=nav_items />
                </nav>

                <div class="social-links">
                    <a
                        href="https://github.com/okawak"
                        target="_blank"
                        rel="noopener noreferrer"
                        title="GitHub"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            width="24"
                            height="24"
                        >
                            <path
                                fill="currentColor"
                                d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.009-.866-.014-1.7-2.782.603-3.369-1.338-3.369-1.338-.455-1.157-1.11-1.465-1.11-1.465-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.891 1.529 2.341 1.089 2.91.833.091-.646.349-1.086.635-1.337-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.03-2.682-.103-.253-.447-1.27.097-2.646 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0112 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.748-1.025 2.748-1.025.546 1.376.202 2.394.1 2.646.64.699 1.026 1.591 1.026 2.682 0 3.841-2.337 4.687-4.565 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.579.688.481C19.138 20.161 22 16.416 22 12c0-5.523-4.477-10-10-10z"
                            />
                        </svg>
                    </a>
                    <a href="_blank" target="_blank" rel="noopener noreferrer" title="Twitter">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            width="24"
                            height="24"
                        >
                            <path
                                fill="currentColor"
                                d="M23.953 4.57a10 10 0 01-2.825.775 4.958 4.958 0 002.163-2.723 10.054 10.054 0 01-3.127 1.184 4.92 4.92 0 00-8.384 4.482C7.69 8.095 4.067 6.13 1.64 3.162a4.822 4.822 0 00-.666 2.475c0 1.71.87 3.213 2.188 4.096a4.904 4.904 0 01-2.228-.616v.06a4.923 4.923 0 003.946 4.827 4.996 4.996 0 01-2.212.085 4.937 4.937 0 004.604 3.417 9.868 9.868 0 01-6.102 2.105c-.39 0-.779-.023-1.17-.067a13.995 13.995 0 007.557 2.209c9.054 0 13.999-7.496 13.999-13.986 0-.209 0-.42-.015-.63a9.936 9.936 0 002.46-2.548l-.047-.02z"
                            />
                        </svg>
                    </a>
                </div>
            </div>
        </header>
    }
}

/// ナビゲーションコンポーネント
#[component]
fn Navigation(
    #[prop(into)] items: Box<dyn Fn() -> Vec<NavigationItem> + Send + 'static>,
) -> impl IntoView {
    view! {
        <ul class="nav-list">
            {move || {
                items()
                    .into_iter()
                    .map(|item| {
                        let active_class = if item.is_active { "active" } else { "" };
                        view! {
                            <li class=active_class>
                                <a href=item.href>{item.title}</a>
                            </li>
                        }
                    })
                    .collect_view()
            }}
        </ul>
    }
}
