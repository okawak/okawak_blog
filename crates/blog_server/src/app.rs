use crate::components::{footer::Footer, header::Header};
use crate::routes::article::ArticlePage;
use crate::routes::category::CategoryPage;
use crate::routes::home::HomePage;
use crate::routes::not_found::NotFoundPage;
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
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.7.2/css/all.min.css"
                />
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
                        view! { <NotFoundPage /> }
                    }>
                        // トップページルート
                        <Route path=StaticSegment("") view=HomePage />

                        // カテゴリーページルート
                        <Route
                            path=StaticSegment("tech")
                            view=move || {
                                view! { <CategoryPage category="tech" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("daily")
                            view=move || {
                                view! { <CategoryPage category="daily" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("statistics")
                            view=move || {
                                view! { <CategoryPage category="statistics" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("physics")
                            view=move || {
                                view! { <CategoryPage category="physics" /> }
                            }
                        />
                        <Route path=path!("/tech/:slug") view=ArticlePage />
                        <Route path=path!("/daily/:slug") view=ArticlePage />
                        <Route path=path!("/statistics/:slug") view=ArticlePage />
                        <Route path=path!("/physics/:slug") view=ArticlePage />
                    </Routes>
                </main>
            </Router>
            <Footer />
        </div>
    }
}
