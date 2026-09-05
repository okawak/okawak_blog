//! Shared HTML shell, metadata, and error views.

use chrono::Datelike;
use topcoat::{
    Result,
    router::StatusCode,
    view::{Child, Unescaped, View, component, view},
};

use crate::assets::{FAVICON, STYLESHEET};

const NOT_FOUND_TITLE: &str = "ページが見つかりません";
const NOT_FOUND_DESCRIPTION: &str = "お探しのページは見つかりませんでした。";

pub(crate) struct ShellMetadata {
    title: String,
    description: String,
    canonical_url: String,
    og_type: &'static str,
}

impl ShellMetadata {
    pub(crate) fn website(title: String, description: String, canonical_url: String) -> Self {
        Self {
            title,
            description,
            canonical_url,
            og_type: "website",
        }
    }

    pub(crate) fn article(title: String, description: String, canonical_url: String) -> Self {
        Self {
            title,
            description,
            canonical_url,
            og_type: "article",
        }
    }
}

#[component]
pub(crate) async fn not_found_page(canonical_path: String) -> Result<impl View> {
    let canonical_url = crate::build_site_url(&canonical_path);

    Ok(view! {
        site_shell(
            status: StatusCode::NOT_FOUND,
            metadata: ShellMetadata::website(
                format!("{NOT_FOUND_TITLE} | {}", crate::SITE_NAME),
                NOT_FOUND_DESCRIPTION.to_string(),
                canonical_url,
            ),
            current_path: canonical_path,
            <div>"ページが見つかりませんでした。"</div>
        )
    })
}

#[component]
pub(crate) async fn article_internal_server_error_page(
    title: String,
    description: String,
    canonical_path: String,
) -> Result<impl View> {
    let canonical_url = crate::build_site_url(&canonical_path);

    Ok(view! {
        site_shell(
            status: StatusCode::INTERNAL_SERVER_ERROR,
            metadata: ShellMetadata::article(title, description, canonical_url),
            current_path: canonical_path,
            <div
                class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground"
            >
                "記事の読み込みに失敗しました"
            </div>
        )
    })
}

#[component]
pub(crate) async fn internal_server_error_page(
    title: String,
    description: String,
    canonical_path: String,
    message: &'static str,
) -> Result<impl View> {
    let canonical_url = crate::build_site_url(&canonical_path);

    Ok(view! {
        site_shell(
            status: StatusCode::INTERNAL_SERVER_ERROR,
            metadata: ShellMetadata::website(title, description, canonical_url),
            current_path: canonical_path,
            <div
                class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground"
            >
                (message)
            </div>
        )
    })
}

#[component]
pub(crate) async fn site_shell(
    status: StatusCode,
    metadata: ShellMetadata,
    current_path: String,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let year = chrono::Local::now().year();
    let math_render_script = Unescaped::new_unchecked(
        r#"
window.okawakRenderMath = function(root) {
  if (!window.katex) return;

  const scope = root || document.body;
  const normalizeExpression = (value) =>
    (value || '').replace(/[\u2009\u200A\u200B\u200C\u200D\u2061\u202F\u2060\uFEFF]/g, '');

  scope.querySelectorAll('.math-inline').forEach((element) => {
    if (element.dataset.katexRendered === 'true') return;

    const expression = normalizeExpression(element.textContent);
    window.katex.render(expression, element, {
      displayMode: false,
      throwOnError: false,
    });
    element.dataset.katexRendered = 'true';
  });

  scope.querySelectorAll('.math-display').forEach((element) => {
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
"#,
    );
    let code_highlight_script = Unescaped::new_unchecked(
        r#"
window.okawakHighlightCode = function(root) {
  if (!window.hljs) return;
  const scope = root || document.body;
  scope.querySelectorAll('.content-prose pre code:not([data-highlighted])')
    .forEach((element) => window.hljs.highlightElement(element));
};
window.okawakScheduleCodeHighlight = function(root) {
  let remaining = 200;
  const attempt = function() {
    if (window.hljs && window.okawakHighlightCode) {
      window.okawakHighlightCode(root);
      return;
    }
    if (remaining > 0) {
      remaining -= 1;
      window.setTimeout(attempt, 50);
    }
  };
  attempt();
};
"#,
    );
    let ShellMetadata {
        title,
        description,
        canonical_url,
        og_type,
    } = metadata;

    Ok(view! {
        (status)
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(title.clone())</title>
                <meta name="description" content=(description.clone())>
                <link rel="canonical" href=(canonical_url.clone())>
                <meta property="og:title" content=(title)>
                <meta property="og:description" content=(description)>
                <meta property="og:url" content=(canonical_url)>
                <meta property="og:type" content=(og_type)>
                <link rel="stylesheet" href=(STYLESHEET)>
                <link
                    rel="icon"
                    href=(FAVICON)
                    type="image/x-icon"
                    sizes="16x16 32x32 48x48"
                >
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.7.2/css/all.min.css"
                >
                <link
                    rel="stylesheet"
                    href="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.css"
                    integrity="sha384-5TcZemv2l/9On385z///+d7MSYlvIEw9FuZTIdZ14vJLqWphw7e7ZPuOiCHJcFCP"
                    crossorigin="anonymous"
                >
                <script
                    defer=""
                    src="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.js"
                    integrity="sha384-cMkvdD8LoxVzGF/RPUKAcvmm49FQ0oxwDF3BGKtDXcEc+T1b2N+teh/OJfpU0jr6"
                    crossorigin="anonymous"
                    onload="window.okawakScheduleMathRender && window.okawakScheduleMathRender();"
                ></script>
                <script>(math_render_script)</script>
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/styles/github-dark.min.css"
                >
                <script
                    defer=""
                    src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js"
                    onload="window.okawakScheduleCodeHighlight && window.okawakScheduleCodeHighlight();"
                ></script>
                <script>(code_highlight_script)</script>
                topcoat::runtime::script()
            </head>
            <body>
                signal menu_open = false;

                <div class="flex min-h-dvh flex-col text-foreground">
                    <header
                        class="sticky top-0 z-50 h-[var(--site-header-height)] border-b border-border/60 bg-[image:var(--site-header-background)] shadow-[0_8px_24px_rgb(0_0_0/0.45)] backdrop-blur-sm"
                    >
                        <div
                            class="relative mx-auto flex h-full max-w-[var(--site-content-width)] items-center justify-between gap-3 px-4 sm:px-6"
                        >
                            <a
                                href="/"
                                class="min-w-0 text-foreground no-underline transition-colors hover:text-primary focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                            >
                                <h1
                                    class="m-0 truncate text-xl leading-tight font-bold sm:text-2xl"
                                >
                                    (crate::SITE_NAME)
                                </h1>
                            </a>

                            <button
                                type="button"
                                class="inline-flex size-10 shrink-0 items-center justify-center rounded-md text-sm font-medium text-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50 md:hidden"
                                aria-controls="site-header-nav"
                                :aria-expanded=$(if menu_open.get() {
                                    "true"
                                } else {
                                    "false"
                                })
                                :aria-label=$(if menu_open.get() {
                                    "ナビゲーションメニューを閉じる"
                                } else {
                                    "ナビゲーションメニューを開く"
                                })
                                @click=$(|_e| menu_open.toggle())
                            >
                                <div
                                    class="flex size-5 flex-col items-center justify-center gap-1.5"
                                    aria-hidden="true"
                                >
                                    <span
                                        :class=$(if menu_open.get() {
                                            "block h-0.5 w-5 translate-y-2 rotate-45 rounded-full bg-current transition-transform"
                                        } else {
                                            "block h-0.5 w-5 rounded-full bg-current transition-all"
                                        })
                                    ></span>
                                    <span
                                        :class=$(if menu_open.get() {
                                            "block h-0.5 w-5 rounded-full bg-current opacity-0 transition-opacity"
                                        } else {
                                            "block h-0.5 w-5 rounded-full bg-current transition-all"
                                        })
                                    ></span>
                                    <span
                                        :class=$(if menu_open.get() {
                                            "block h-0.5 w-5 -translate-y-2 -rotate-45 rounded-full bg-current transition-transform"
                                        } else {
                                            "block h-0.5 w-5 rounded-full bg-current transition-all"
                                        })
                                    ></span>
                                </div>
                            </button>

                            <nav
                                id="site-header-nav"
                                aria-label="メインナビゲーション"
                                :class=$(if menu_open.get() {
                                    "flex absolute inset-x-4 top-[calc(100%+0.5rem)] flex-col gap-3 rounded-lg border border-border bg-card/98 p-4 shadow-[0_18px_36px_rgb(0_0_0/0.55)] backdrop-blur-sm md:static md:flex md:flex-row md:items-center md:gap-6 md:border-0 md:bg-transparent md:p-0 md:shadow-none"
                                } else {
                                    "hidden absolute inset-x-4 top-[calc(100%+0.5rem)] flex-col gap-3 rounded-lg border border-border bg-card/98 p-4 shadow-[0_18px_36px_rgb(0_0_0/0.55)] backdrop-blur-sm md:static md:flex md:flex-row md:items-center md:gap-6 md:border-0 md:bg-transparent md:p-0 md:shadow-none"
                                })
                            >
                                <ul
                                    class="m-0 flex list-none flex-col gap-1 p-0 md:flex-row md:items-center md:gap-2"
                                >
                                    <li>
                                        <a
                                            href="/"
                                            aria-current=(if current_path == "/" {
                                                Some("page")
                                            } else {
                                                None
                                            })
                                            class=(if current_path == "/" {
                                                "block rounded-md border-b-2 border-primary px-3 py-2 text-sm font-medium text-foreground no-underline"
                                            } else {
                                                "block rounded-md border-b-2 border-transparent px-3 py-2 text-sm font-medium text-muted-foreground no-underline transition-colors hover:border-primary hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                                            })
                                            @click=$(|_e| menu_open.set(false))
                                        >
                                            "ホーム"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="/about"
                                            aria-current=(if current_path == "/about" {
                                                Some("page")
                                            } else {
                                                None
                                            })
                                            class=(if current_path == "/about" {
                                                "block rounded-md border-b-2 border-primary px-3 py-2 text-sm font-medium text-foreground no-underline"
                                            } else {
                                                "block rounded-md border-b-2 border-transparent px-3 py-2 text-sm font-medium text-muted-foreground no-underline transition-colors hover:border-primary hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                                            })
                                            @click=$(|_e| menu_open.set(false))
                                        >
                                            "About"
                                        </a>
                                    </li>
                                </ul>

                                <div
                                    class="border-t border-border pt-3 md:border-t-0 md:pt-0"
                                >
                                    <a
                                        href="https://github.com/okawak"
                                        class="inline-flex size-10 items-center justify-center rounded-md text-foreground transition-colors hover:bg-accent hover:text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                                        aria-label="Open okawak GitHub profile"
                                        rel="noopener noreferrer"
                                        target="_blank"
                                    >
                                        <i class="fab fa-github text-xl" aria-hidden="true"></i>
                                    </a>
                                </div>
                            </nav>
                        </div>
                    </header>
                    <main class="content-container flex-1">(child)</main>
                    <footer
                        class="border-t border-border bg-gradient-to-r from-card to-background px-4 py-8 text-center text-sm text-muted-foreground"
                    >
                        <div class="mx-auto max-w-[var(--site-content-width)]">
                            <p class="my-2 leading-relaxed">
                                (format!("© {year} okawak. All Rights Reserved."))
                            </p>
                            <p class="my-2 leading-relaxed">
                                <small>
                                    "Powered by "
                                    <a
                                        href="https://github.com/tokio-rs/topcoat"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        "Topcoat"
                                    </a>
                                </small>
                            </p>
                        </div>
                    </footer>
                </div>
            </body>
        </html>
    })
}
