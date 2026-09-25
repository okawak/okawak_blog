//! Shared HTML shell, metadata, and error views.

use crate::i18n::{Message, japanese_path, locale_from_path, t};
use chrono::Datelike;
use domain::{Locale, SiteLocalesDocument};
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{StatusCode, request},
    runtime::{Event, signal},
    view::{Child, Unescaped, View, attributes, class, component, view},
};

use crate::assets::{FAVICON, STYLESHEET};
use crate::components::{
    alert::{AlertVariant, alert, alert_description, alert_title},
    button::{ButtonSize, ButtonVariant, button, button_variants},
    tooltip::{TooltipSide, tooltip, tooltip_content},
};
use crate::icons::GITHUB;

pub(crate) struct ShellMetadata {
    title: String,
    description: String,
    canonical_path: String,
    og_type: &'static str,
    locale: Locale,
    locales: SiteLocalesDocument,
}

impl ShellMetadata {
    pub(crate) fn website(
        locale: Locale,
        title: String,
        description: String,
        canonical_path: String,
    ) -> Self {
        Self {
            title,
            description,
            canonical_path,
            og_type: "website",
            locale,
            locales: Default::default(),
        }
    }
    pub(crate) fn article(
        locale: Locale,
        title: String,
        description: String,
        canonical_path: String,
    ) -> Self {
        Self {
            og_type: "article",
            ..Self::website(locale, title, description, canonical_path)
        }
    }
    pub(crate) fn with_locales(mut self, locales: SiteLocalesDocument) -> Self {
        self.locales = locales;
        self
    }
}

#[component]
pub(crate) async fn not_found_page(canonical_path: String) -> Result<impl View> {
    let locale = locale_from_path(&canonical_path);
    Ok(view! {
        site_shell(
            status: StatusCode::NOT_FOUND,
            metadata: ShellMetadata::website(
                locale,
                format!(
                    "{} | {}",
                    t(locale, Message::ErrorNotFoundTitle),
                    t(locale, Message::SiteName),
                ),
                t(locale, Message::ErrorNotFoundDescription).to_string(),
                canonical_path,
            ),
            status_notice(
                variant: AlertVariant::Neutral,
                title: Some(t(locale, Message::ErrorNotFoundTitle)),
                message: t(locale, Message::ErrorNotFoundBody)
            )
        )
    })
}

#[component]
pub(crate) async fn article_internal_server_error_page(
    title: String,
    description: String,
    canonical_path: String,
) -> Result<impl View> {
    let locale = locale_from_path(&canonical_path);
    Ok(view! {
        site_shell(
            status: StatusCode::INTERNAL_SERVER_ERROR,
            metadata: ShellMetadata::article(locale, title, description, canonical_path),
            status_notice(
                variant: AlertVariant::Destructive,
                title: None,
                message: t(locale, Message::ErrorArticle)
            )
        )
    })
}

#[component]
pub(crate) async fn internal_server_error_page(
    title: String,
    description: String,
    canonical_path: String,
    message: Message,
) -> Result<impl View> {
    let locale = locale_from_path(&canonical_path);
    Ok(view! {
        site_shell(
            status: StatusCode::INTERNAL_SERVER_ERROR,
            metadata: ShellMetadata::website(locale, title, description, canonical_path),
            status_notice(
                variant: AlertVariant::Destructive,
                title: None,
                message: t(locale, message)
            )
        )
    })
}

#[component]
async fn status_notice(
    variant: AlertVariant,
    title: Option<&str>,
    message: &str,
) -> Result<impl View> {
    Ok(view! {
        alert(
            variant: variant,
            attrs: attributes! {
                class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] bg-secondary p-8 text-center"
            },
            if let Some(title) = title {
                alert_title((title))
            }
            alert_description((message))
        )
    })
}

#[component]
pub(crate) async fn site_shell(
    cx: &Cx,
    status: StatusCode,
    metadata: ShellMetadata,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let locale = metadata.locale;
    let home_path = locale.path("/");
    let home_href = if locale == Locale::Ja {
        crate::language::choice_href(&home_path, locale)
    } else {
        home_path.clone()
    };
    let about_href = locale.path("/about");
    let requested_path = request::uri(cx).path();
    let home_is_current = requested_path.trim_end_matches('/') == home_path.trim_end_matches('/');
    let about_is_current = requested_path == about_href;
    let show_about = locale == Locale::Ja || metadata.locales.path("/about", locale).is_some();
    let available = metadata
        .locales
        .routes
        .get(japanese_path(&metadata.canonical_path));
    let alternates = available
        .into_iter()
        .flatten()
        .map(|other| (*other, other.path(japanese_path(&metadata.canonical_path))))
        .collect::<Vec<_>>();
    let menu_open_label = t(locale, Message::NavOpen).to_string();
    let switch_locales = if status != StatusCode::OK {
        crate::app::page_loader(cx)
            .loader()
            .load_locales()
            .await
            .unwrap_or_default()
    } else {
        metadata.locales.clone()
    };
    let menu_close_label = t(locale, Message::NavClose).to_string();
    let year = chrono::Local::now().year();
    let menu_open = signal(cx, || false);
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
        canonical_path,
        og_type,
        ..
    } = metadata;
    let canonical_url = crate::build_site_url(&canonical_path);

    Ok(view! {
        (status)
        <!DOCTYPE html>
        <html lang=(locale.as_str())>
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
                <meta
                    property="og:locale"
                    content=(if locale == Locale::Ja { "ja_JP" } else { "en_US" })
                >
                for (other, path) in &alternates {
                    <link
                        rel="alternate"
                        hreflang=(other.as_str())
                        href=(crate::build_site_url(path))
                    >
                }
                <link rel="stylesheet" href=(STYLESHEET)>
                <link
                    rel="stylesheet"
                    href="https://fonts.googleapis.com/css2?family=Noto+Sans+JP:wght@400..700&display=swap"
                >
                <link
                    rel="icon"
                    href=(FAVICON)
                    type="image/x-icon"
                    sizes="16x16 32x32 48x48"
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
                topcoat::dev::script()
            </head>
            <body>
                <div class="flex min-h-dvh flex-col text-foreground">
                    <header
                        class="sticky top-0 z-50 h-[var(--site-header-height)] border-b border-border/60 bg-[image:var(--site-header-background)] shadow-[0_8px_24px_rgb(0_0_0/0.45)] backdrop-blur-sm"
                    >
                        <div
                            class="relative mx-auto flex h-full max-w-[var(--site-content-width)] items-center justify-between gap-3 px-4 sm:px-6"
                        >
                            <a
                                href=(home_href.clone())
                                class="mr-auto min-w-0 text-foreground no-underline transition-colors hover:text-primary focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                            >
                                <h1
                                    class="m-0 truncate text-xl leading-tight font-bold sm:text-2xl"
                                >
                                    (t(locale, Message::SiteName))
                                </h1>
                            </a>

                            crate::language::language_switcher(
                                locale: locale,
                                path: &canonical_path,
                                locales: &switch_locales
                            )

                            button(
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Icon,
                                attrs: attributes! {
                                    type="button"
                                    class="md:hidden"
                                    aria-controls="site-header-nav"
                                    :aria-expanded=$(if menu_open.get() {
                                        "true"
                                    } else {
                                        "false"
                                    })
                                    :aria-label=$(if menu_open.get() {
                                        menu_close_label.clone()
                                    } else {
                                        menu_open_label.clone()
                                    })
                                    @click=$(|_event: Event| menu_open.toggle())
                                },
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
                            )

                            <nav
                                id="site-header-nav"
                                aria-label=(t(locale, Message::NavMain))
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
                                            href=(home_href)
                                            aria-current=(home_is_current.then_some("page"))
                                            class=(class!(
                                                button_variants(ButtonVariant::Ghost, ButtonSize::Sm),
                                                "w-full justify-start no-underline md:w-auto",
                                                if home_is_current {
                                                    "bg-foreground/5 text-primary"
                                                } else {
                                                    "text-muted-foreground hover:text-foreground"
                                                },
                                            ))
                                            @click=$(|_e| menu_open.set(false))
                                        >
                                            (t(locale, Message::NavHome))
                                        </a>
                                    </li>
                                    if show_about {
                                        <li>
                                            <a
                                                href=(about_href)
                                                aria-current=(about_is_current.then_some("page"))
                                                class=(class!(
                                                    button_variants(ButtonVariant::Ghost, ButtonSize::Sm),
                                                    "w-full justify-start no-underline md:w-auto",
                                                    if about_is_current {
                                                        "bg-foreground/5 text-primary"
                                                    } else {
                                                        "text-muted-foreground hover:text-foreground"
                                                    },
                                                ))
                                                @click=$(|_e| menu_open.set(false))
                                            >
                                                (t(locale, Message::NavAbout))
                                            </a>
                                        </li>
                                    }
                                </ul>

                                <div
                                    class="border-t border-border pt-3 md:border-t-0 md:pt-0"
                                >
                                    tooltip(
                                        <a
                                            href="https://github.com/okawak"
                                            class=(button_variants(
                                                ButtonVariant::Ghost,
                                                ButtonSize::Icon,
                                            ))
                                            aria-label=(t(locale, Message::NavGithub))
                                            rel="noopener noreferrer"
                                            target="_blank"
                                        >
                                            icon(data: GITHUB, size: 20)
                                        </a>
                                        tooltip_content(
                                            side: TooltipSide::Bottom,
                                            (t(locale, Message::NavGithub))
                                        )
                                    )
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
                                (crate::i18n::interpolate(
                                    locale,
                                    Message::FooterCopyright,
                                    &[("year", year.to_string())],
                                ))
                            </p>
                            <p class="my-2 leading-relaxed">
                                <small>
                                    (t(locale, Message::FooterPowered))
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
