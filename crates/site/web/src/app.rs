use crate::SITE_NAME;
use crate::components::{footer::Footer, header::Header};
use crate::routes::about::AboutPage;
use crate::routes::article::ArticlePage;
use crate::routes::category::CategoryPage;
use crate::routes::home::HomePage;
use crate::routes::not_found::NotFoundPage;
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

/// Shell function used for server-side rendering.
/// This function renders the full HTML document.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                // Load Font Awesome from the CDN.
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.7.2/css/all.min.css"
                />

                // Load KaTeX from the CDN.
                <link
                    rel="stylesheet"
                    href="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.css"
                    integrity="sha384-5TcZemv2l/9On385z///+d7MSYlvIEw9FuZTIdZ14vJLqWphw7e7ZPuOiCHJcFCP"
                    crossorigin="anonymous"
                />
                <script
                    // Load asynchronously.
                    defer
                    src="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.js"
                    integrity="sha384-cMkvdD8LoxVzGF/RPUKAcvmm49FQ0oxwDF3BGKtDXcEc+T1b2N+teh/OJfpU0jr6"
                    crossorigin="anonymous"
                ></script>
                <script
                    defer
                    src="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/contrib/auto-render.min.js"
                    integrity="sha384-hCXGrW6PitJEwbkoStFjeJxv+fSOOQKOPbJxSfM6G5sWZjAyWhXiTIIAmQqnlLlh"
                    crossorigin="anonymous"
                ></script>
                // <script>
                // document.addEventListener("DOMContentLoaded", function() {
                // renderMathInElement(document.body, {
                // delimiters: [
                // {left: "$$", right: "$$", display: true},
                // {left: "$", right: "$", display: false}
                // ],
                // });
                // })
                // </script>

                // Load highlight.js from the CDN.
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/styles/default.min.css"
                />
                <script
                    defer
                    src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js"
                ></script>
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

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/web.css" />

        // sets the document title
        <Title text=SITE_NAME />

        <div class="app-container">
            <Router>
                <Header />
                <main class="content-container">
                    <Routes fallback=|| {
                        view! { <NotFoundPage /> }
                    }>
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/about") view=AboutPage />
                        <Route path=path!("/articles/:slug") view=ArticlePage />
                        <Route path=path!("/categories/:category") view=CategoryPage />
                    </Routes>
                </main>
            </Router>
            <Footer />
        </div>
    }
}
