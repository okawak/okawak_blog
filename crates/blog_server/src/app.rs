use crate::components::{footer::Footer, header::Header};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path, StaticSegment,
};

/// サーバーサイドレンダリングのためのシェル関数
/// この関数はHTMLドキュメント全体を生成します
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// メインのアプリケーションコンポーネント
#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/blog_server.css" />

        // sets the document title
        <Title text="ぶくせんの探窟メモ" />

        <div class="app-container">
            <Router>
                <Header />
                <main class="content-container">
                    <Routes fallback=|| {
                        view! {
                            <div class="not-found">ページが見つかりませんでした</div>
                        }
                    }>
                        // トップページルート
                        <Route path=StaticSegment("") view=crate::routes::home::HomePage />

                        // カテゴリーページルート
                        <Route
                            path=StaticSegment("tech")
                            view=move || {
                                view! { <crate::routes::category::CategoryPage category="tech" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("daily")
                            view=move || {
                                view! { <crate::routes::category::CategoryPage category="daily" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("statistics")
                            view=move || {
                                view! {
                                    <crate::routes::category::CategoryPage category="statistics" />
                                }
                            }
                        />
                        <Route
                            path=StaticSegment("physics")
                            view=move || {
                                view! {
                                    <crate::routes::category::CategoryPage category="physics" />
                                }
                            }
                        />
                        <Route path=path!("/tech/:slug") view=crate::routes::article::ArticlePage />
                        <Route
                            path=path!("/daily/:slug")
                            view=crate::routes::article::ArticlePage
                        />
                        <Route
                            path=path!("/statistics/:slug")
                            view=crate::routes::article::ArticlePage
                        />
                        <Route
                            path=path!("/physics/:slug")
                            view=crate::routes::article::ArticlePage
                        />
                    </Routes>
                </main>
            </Router>
            <Footer />
        </div>
    }
}
