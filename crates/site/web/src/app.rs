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
                    onload="window.okawakScheduleMathRender && window.okawakScheduleMathRender();"
                ></script>
                <script>
                    {r#"
                    window.okawakRenderMath = function(root) {
                    if (!window.katex) return;
                    
                    const scope = root || document.body;
                    const normalizeExpression = (value) =>
                    (value || '').replace(/[  ​‌‍⁡ ⁠﻿]/g, '');
                    
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
                    
                    window.okawakScheduleMathRender = function(root) {
                    let remaining = 200;
                    const attempt = function() {
                    if (window.katex && window.okawakRenderMath) {
                    window.okawakRenderMath(root);
                    return;
                    }
                    
                    if (remaining > 0) {
                    remaining -= 1;
                    window.setTimeout(attempt, 50);
                    }
                    };
                    
                    attempt();
                    };
                    
                    document.addEventListener('DOMContentLoaded', function() {
                    if (window.okawakScheduleMathRender) {
                    window.okawakScheduleMathRender();
                    }
                    });
                    
                    window.addEventListener('load', function() {
                    if (window.okawakScheduleMathRender) {
                    window.okawakScheduleMathRender();
                    }
                    });
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

    #[cfg(target_arch = "wasm32")]
    let render_scheduled = std::rc::Rc::new(std::cell::Cell::new(false));
    #[cfg(target_arch = "wasm32")]
    let render_scheduled_for_location = render_scheduled.clone();

    Effect::new(move |_| {
        let _ = location.pathname.get();

        #[cfg(target_arch = "wasm32")]
        schedule_math_render(&render_scheduled_for_location);

        #[cfg(not(target_arch = "wasm32"))]
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
            let root = document
                .query_selector("main.content-container")
                .ok()
                .flatten()
                .map(Into::into)
                .or_else(|| document.body().map(Into::into));
            let Some(root) = root else {
                return;
            };
            let render_scheduled = render_scheduled.clone();

            let callback = Closure::wrap(Box::new(
                move |records: js_sys::Array, _observer: MutationObserver| {
                    let should_render = records.iter().any(|record| {
                        record
                            .dyn_into::<web_sys::MutationRecord>()
                            .ok()
                            .is_some_and(|record| mutation_record_contains_math(&record))
                    });

                    if should_render {
                        schedule_math_render(&render_scheduled);
                    }
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
                .observe_with_options(&root, &options)
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
    use js_sys::{Function, Reflect};
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};

    fn call_render_math(window: &web_sys::Window) -> bool {
        let render_math = Reflect::get(
            window.as_ref(),
            &JsValue::from_str("okawakScheduleMathRender"),
        )
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .or_else(|| {
            Reflect::get(window.as_ref(), &JsValue::from_str("okawakRenderMath"))
                .ok()
                .and_then(|value| value.dyn_into::<Function>().ok())
        });

        if let Some(render_math) = render_math {
            let _ = render_math.call0(window.as_ref());
            true
        } else {
            false
        }
    }

    if let Some(window) = web_sys::window() {
        if call_render_math(&window) {
            return;
        }

        let remaining = Rc::new(RefCell::new(20_u32));
        let retry = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
        let retry_for_closure = Rc::clone(&retry);
        let remaining_for_closure = Rc::clone(&remaining);
        let window_for_closure = window.clone();

        *retry.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if call_render_math(&window_for_closure) {
                let _ = retry_for_closure.borrow_mut().take();
                return;
            }

            let mut remaining = remaining_for_closure.borrow_mut();
            if *remaining == 0 {
                let _ = retry_for_closure.borrow_mut().take();
                return;
            }

            *remaining -= 1;

            if let Some(callback) = retry_for_closure.borrow().as_ref() {
                let _ = window_for_closure.set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    50,
                );
            }
        }) as Box<dyn FnMut()>));

        if let Some(callback) = retry.borrow().as_ref() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                50,
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn schedule_math_render(render_scheduled: &std::rc::Rc<std::cell::Cell<bool>>) {
    use wasm_bindgen::{JsCast, closure::Closure};

    if render_scheduled.get() {
        return;
    }

    let Some(window) = web_sys::window() else {
        return;
    };

    render_scheduled.set(true);
    let render_scheduled = render_scheduled.clone();
    let callback = Closure::once(move || {
        render_scheduled.set(false);
        trigger_math_render();
    });

    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        0,
    );
    callback.forget();
}

#[cfg(target_arch = "wasm32")]
fn mutation_record_contains_math(record: &web_sys::MutationRecord) -> bool {
    let nodes = record.added_nodes();
    (0..nodes.length()).any(|index| nodes.item(index).as_ref().is_some_and(node_contains_math))
}

#[cfg(target_arch = "wasm32")]
fn node_contains_math(node: &web_sys::Node) -> bool {
    use wasm_bindgen::JsCast;

    node.dyn_ref::<web_sys::Element>().is_some_and(|element| {
        element.class_list().contains("okawak-katex-inline")
            || element.class_list().contains("okawak-katex-display")
            || element
                .query_selector(".okawak-katex-inline, .okawak-katex-display")
                .ok()
                .flatten()
                .is_some()
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_math_render() {}
