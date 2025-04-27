use crate::components::{footer::Footer, header::Header};
use crate::routes::article::ArticlePage;
use crate::routes::category::CategoryPage;
use crate::routes::home::HomePage;
use crate::routes::not_found::NotFoundPage;
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
    path,
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
                            view=|| {
                                view! { <CategoryPage category="tech" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("daily")
                            view=|| {
                                view! { <CategoryPage category="daily" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("statistics")
                            view=|| {
                                view! { <CategoryPage category="statistics" /> }
                            }
                        />
                        <Route
                            path=StaticSegment("physics")
                            view=|| {
                                view! { <CategoryPage category="physics" /> }
                            }
                        />
                        <Route
                            path=path!("/tech/:slug")
                            view=|| {
                                view! { <ArticlePage cat="tech" /> }
                            }
                        />
                        <Route
                            path=path!("/daily/:slug")
                            view=|| {
                                view! { <ArticlePage cat="daily" /> }
                            }
                        />
                        <Route
                            path=path!("/statistics/:slug")
                            view=|| {
                                view! { <ArticlePage cat="statistics" /> }
                            }
                        />
                        <Route
                            path=path!("/physics/:slug")
                            view=|| {
                                view! { <ArticlePage cat="physics" /> }
                            }
                        />
                    </Routes>
                </main>
            </Router>
            <Footer />
        </div>
    }
}
