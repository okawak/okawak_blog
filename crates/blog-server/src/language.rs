//! Browser language preference at the site's entry point; page URLs keep their locale.

use blog_domain::{Locale, SiteLocalesDocument};
use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies, time::Duration},
    l10n::{accept_language, negotiate},
    router::{
        HeaderValue, Uri,
        error::{redirect as temporary_redirect, see_other},
        header, request,
        response::{IntoResponse, Response},
    },
    runtime::{PrefetchMode, link_attrs},
    view::{View, attributes, component, element_id, view},
};

use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button_variants},
        floating::FloatingAlign,
        popover::{popover, popover_content, popover_trigger},
    },
    i18n::{Message, route_key, t},
};

const COOKIE: &str = "okawak_locale";

pub(crate) fn explicit_choice(uri: &Uri) -> Option<Locale> {
    // Let the router canonicalize a trailing slash before applying the language choice.
    if uri.path() != "/" && uri.path().ends_with('/') {
        return None;
    }
    let mut choices = uri
        .query()?
        .split('&')
        .filter_map(|part| part.strip_prefix("lang="));
    let choice = choices.next()?.parse().ok()?;
    choices.next().is_none().then_some(choice)
}

pub(crate) fn legacy_location(uri: &Uri) -> Option<String> {
    let mut segments = uri
        .path()
        .trim_end_matches('/')
        .strip_prefix('/')?
        .splitn(2, '/');
    let first = percent_encoding::percent_decode_str(segments.next()?)
        .decode_utf8()
        .ok()?;
    let article = segments.next();
    let route = if first == "about" && article.is_none() {
        "/about".to_owned()
    } else {
        let category: blog_domain::Category = first.parse().ok()?;
        match article {
            None => blog_domain::build_category_path(&category),
            Some(article) => {
                let decoded = percent_encoding::percent_decode_str(article)
                    .decode_utf8()
                    .ok()?;
                let slug: blog_domain::Slug = decoded.trim_suffix(".html").parse().ok()?;
                blog_domain::build_article_path(&category, &slug)
            }
        }
    };
    let mut target = Locale::Ja.path(&route);
    if let Some(query) = uri.query() {
        target.push('?');
        target.push_str(query);
    }
    Some(target)
}

fn preferred_locale(cx: &Cx, locales: &SiteLocalesDocument) -> Locale {
    let available = locales
        .routes
        .get("/")
        .map(Vec::as_slice)
        .unwrap_or(&[Locale::Ja]);
    let supported: Vec<_> = available
        .iter()
        .copied()
        .map(crate::format::formatting_locale)
        .collect();
    negotiate(accept_language(cx), &supported)
        .and_then(|chosen| supported.iter().position(|item| item == chosen))
        .map(|i| available[i])
        .unwrap_or(if available.contains(&Locale::En) {
            Locale::En
        } else {
            Locale::Ja
        })
}

fn native_name(locale: Locale) -> &'static str {
    match locale {
        Locale::Ja => "日本語",
        Locale::En => "English",
        Locale::ZhHans => "简体中文",
    }
}

fn destination(locales: &SiteLocalesDocument, path: &str, locale: Locale) -> Option<String> {
    locales
        .path(route_key(path), locale)
        .or_else(|| locales.path("/", locale))
        // Error pages can still link home if the locale catalog is unavailable.
        .or_else(|| {
            (locale == Locale::Ja && locales.routes.is_empty()).then(|| Locale::Ja.path("/"))
        })
}

pub(crate) fn vary_home(response: &mut Response) {
    response.headers_mut().insert(
        header::VARY,
        HeaderValue::from_static("Accept-Language, Cookie"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache"),
    );
}

pub(crate) async fn redirect(cx: &Cx, explicit: Option<Locale>) -> Option<Response> {
    let uri = request::uri(cx);
    let locales = match crate::app::page_loader(cx).load_locales().await {
        Ok(locales) => locales,
        Err(error) => {
            tracing::error!(%error, "language selection could not read the published locales");
            if uri.path() != "/" {
                return None;
            }
            Default::default()
        }
    };
    let explicit = explicit.filter(|locale| destination(&locales, "/", *locale).is_some());
    if explicit.is_none() && uri.path() != "/" {
        return None;
    }
    let locale = explicit
        .or_else(|| {
            cookies(cx)
                .get(COOKIE)
                .and_then(|cookie| cookie.value().parse().ok())
                .filter(|locale| destination(&locales, "/", *locale).is_some())
        })
        .unwrap_or_else(|| preferred_locale(cx, &locales));
    let mut location = destination(&locales, uri.path(), locale)
        .or_else(|| (uri.path() == "/").then(|| Locale::Ja.path("/")))?;
    let query = uri
        .query()
        .unwrap_or_default()
        .split('&')
        .filter(|part| !part.is_empty() && !part.starts_with("lang="))
        .collect::<Vec<_>>()
        .join("&");
    if !query.is_empty() {
        location.push('?');
        location.push_str(&query);
    }
    let mut response = if explicit.is_some() {
        see_other(&location).into_response(cx)
    } else {
        temporary_redirect(&location).into_response(cx)
    }
    .ok()?;
    response.headers_mut().insert(
        header::VARY,
        HeaderValue::from_static("Accept-Language, Cookie"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if explicit.is_some() {
        cookies(cx).add(
            Cookie::build((COOKIE, locale.to_string()))
                .path("/")
                .max_age(Duration::days(365))
                .http_only(true)
                .same_site(SameSite::Lax)
                .build(),
        );
    }
    Some(response)
}

#[component]
pub(crate) async fn language_switcher(
    cx: &Cx,
    locale: Locale,
    path: &str,
    locales: &SiteLocalesDocument,
) -> topcoat::Result<impl View> {
    let panel_id = element_id(&cx.keyed(path));
    Ok(view! {
        popover(
            popover_trigger(
                target: &panel_id,
                attrs: attributes! {
                    aria-label=(format!(
                        "{} {}",
                        t(locale, Message::NavLanguage),
                        native_name(locale),
                    ))
                    class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
                },
                <span class="max-w-24 truncate">(native_name(locale))</span>
                <span
                    aria-hidden="true"
                    class="size-2 shrink-0 -translate-y-0.5 rotate-45 border-r border-b border-current group-has-[:popover-open]/popover:translate-y-0.5 group-has-[:popover-open]/popover:rotate-225"
                ></span>
            )
            popover_content(
                id: &panel_id,
                align: FloatingAlign::End,
                attrs: attributes! {
                    role="group"
                    aria-label=(t(locale, Message::NavLanguage))
                    class="max-w-[min(11rem,calc(100vw-2rem))] max-h-[calc(100dvh-7rem)] overflow-y-auto p-1.5! text-left text-sm"
                },
                #[key(other.as_str())]
                for other in Locale::ALL {
                    if let Some(target) = destination(locales, path, other) {
                        <a
                            (link_attrs(
                                cx,
                                format!("{target}?lang={other}"),
                                PrefetchMode::Never,
                            ))
                            lang=(other.as_str())
                            aria-current=((other == locale).then_some("true"))
                            class="flex min-h-11 items-center justify-between gap-2 rounded-md px-3 text-foreground no-underline hover:bg-accent focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring aria-current:bg-accent aria-current:font-semibold"
                        >
                            (native_name(other))
                            if other == locale {
                                <span aria-hidden="true" class="text-primary">"✓"</span>
                            }
                        </a>
                    } else {
                        <span
                            lang=(other.as_str())
                            aria-disabled="true"
                            class="flex min-h-11 items-center px-3 text-muted-foreground"
                        >
                            (native_name(other))
                        </span>
                    }
                }
            )
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_translation_uses_its_home_but_missing_language_is_unavailable() {
        let catalog: SiteLocalesDocument = serde_json::from_str(
            r#"{
            "schema_version": 1,
            "routes": {"/": ["ja", "en"], "/tech/japanese-only": ["ja"]}
        }"#,
        )
        .unwrap();
        assert_eq!(
            destination(&catalog, "/tech/japanese-only", Locale::En).as_deref(),
            Some("/en")
        );
        assert_eq!(
            destination(&catalog, "/tech/japanese-only", Locale::Ja).as_deref(),
            Some("/ja/tech/japanese-only")
        );
        assert_eq!(
            destination(&SiteLocalesDocument::default(), "/", Locale::En),
            None
        );
    }

    #[test]
    fn manual_choice_is_exact_and_unambiguous() {
        for query in [
            "lang=fr",
            "lang=ja&lang=en",
            "language=ja",
            "lang=",
            "lang=%65n",
        ] {
            assert_eq!(
                explicit_choice(&format!("/?{query}").parse().unwrap()),
                None
            );
        }
        assert_eq!(
            explicit_choice(&"/?lang=ja&from=nav".parse().unwrap()),
            Some(Locale::Ja)
        );
    }
}
