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
    components::{FlatRoutes, Route, Router},
    hooks::use_location,
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
                <script>
                    {r#"
                    window.okawakRenderMath = function(root) {
                    if (!window.katex) return;
                    
                    const scope = root || document.body;
                    const normalizeExpression = (value) =>
                     (value || '').replace(/[\u2009\u200A\u200B\u200C\u200D\u2061\u202F\u2060\uFEFF]/g, '');
                    
                    scope.querySelectorAll('.okawak-katex-inline').forEach((element) => {
                    if (element.dataset.katexRendered === 'true') return;
                    
                    const expression = normalizeExpression(element.textContent);
                    window.katex.render(expression, element, {
                      displayMode: false,
                      throwOnError: false,
                    });
                    element.dataset.katexRendered = 'true';
                    });
                    
                    scope.querySelectorAll('.okawak-katex-display').forEach((element) => {
                    if (element.dataset.katexRendered === 'true') return;
                    
                    const expression = normalizeExpression(element.textContent);
                    window.katex.render(expression, element, {
                      displayMode: true,
                      throwOnError: false,
                    });
                    element.dataset.katexRendered = 'true';
                    });
                    };
                    
                    "#}
                </script>

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
                <MathRenderer />
                <Header />
                <main class="content-container">
                    <FlatRoutes fallback=|| {
                        view! { <NotFoundPage /> }
                    }>
                        <Route path=path!("") view=HomePage />
                        <Route path=path!("about") view=AboutPage />
                        <Route path=path!(":category/:slug") view=ArticlePage />
                        <Route path=path!(":category") view=CategoryPage />
                    </FlatRoutes>
                </main>
            </Router>
            <Footer />
        </div>
    }
}

#[component]
fn MathRenderer() -> impl IntoView {
    let location = use_location();

    Effect::new(move |_| {
        let _ = location.pathname.get();
        trigger_math_render();
    });

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::prelude::on_cleanup;
        use wasm_bindgen::{JsCast, closure::Closure};
        use web_sys::{MutationObserver, MutationObserverInit};

        let observer = StoredValue::new_local(
            None::<(
                MutationObserver,
                Closure<dyn FnMut(js_sys::Array, MutationObserver)>,
            )>,
        );

        Effect::new(move |_| {
            if observer.with_value(|value| value.is_some()) {
                return;
            }

            let Some(window) = web_sys::window() else {
                return;
            };
            let Some(document) = window.document() else {
                return;
            };
            let Some(body) = document.body() else {
                return;
            };

            let callback = Closure::wrap(Box::new(
                move |_records: js_sys::Array, _observer: MutationObserver| {
                    trigger_math_render();
                },
            )
                as Box<dyn FnMut(js_sys::Array, MutationObserver)>);
            let observer_instance = MutationObserver::new(callback.as_ref().unchecked_ref()).ok();

            let Some(observer_instance) = observer_instance else {
                return;
            };

            let options = MutationObserverInit::new();
            options.set_child_list(true);
            options.set_subtree(true);

            if observer_instance
                .observe_with_options(&body, &options)
                .is_ok()
            {
                observer.set_value(Some((observer_instance, callback)));
            }
        });

        on_cleanup(move || {
            observer.update_value(|value| {
                if let Some((observer_instance, _callback)) = value.take() {
                    observer_instance.disconnect();
                }
            });
        });
    }

    view! { <></> }
}

#[cfg(target_arch = "wasm32")]
fn trigger_math_render() {
    use js_sys::Function;

    if let Some(window) = web_sys::window() {
        let callback = Function::new_no_args(
            r#"
            (function retryRenderMath(remaining) {
                if (window.katex && window.okawakRenderMath) {
                    window.okawakRenderMath();
                    return;
                }

                if (remaining > 0) {
                    window.setTimeout(function () {
                        retryRenderMath(remaining - 1);
                    }, 50);
                }
            })(20);
            "#,
        );
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&callback, 0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_math_render() {}
