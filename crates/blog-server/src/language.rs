//! Browser language preference at the site's entry point; page URLs keep their locale.

use blog_domain::{Locale, SiteLocalesDocument};
use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies, time::Duration},
    l10n::{accept_language, locale, negotiate},
    router::{
        HeaderValue, Uri,
        error::{redirect as temporary_redirect, see_other},
        header, request,
        response::{IntoResponse, Response},
    },
    runtime::{PrefetchMode, link_attrs},
    view::{View, attributes, component, view},
};

use crate::{
    components::toggle::{ToggleSize, toggle_group, toggle_link},
    i18n::{Message, japanese_path, t},
};

const COOKIE: &str = "okawak_locale";

pub(crate) fn is_negotiated_request(uri: &Uri) -> bool {
    // Preserve the router's canonicalization (and its query) before selecting a translation.
    uri.path() == "/" || (!uri.path().ends_with('/') && explicit_choice(uri).is_some())
}

fn explicit_choice(uri: &Uri) -> Option<Locale> {
    let mut choices = uri
        .query()?
        .split('&')
        .filter_map(|part| part.strip_prefix("lang="));
    let choice = choices.next()?.parse().ok()?;
    choices.next().is_none().then_some(choice)
}

fn preferred_locale(cx: &Cx) -> Locale {
    match negotiate(accept_language(cx), &[locale!("ja"), locale!("en")]) {
        Some(locale) if locale == &locale!("ja") => Locale::Ja,
        _ => Locale::En,
    }
}

fn destination(locales: &SiteLocalesDocument, path: &str, locale: Locale) -> Option<String> {
    locales
        .path(japanese_path(path), locale)
        .or_else(|| locales.path("/", locale))
        // Old releases and error pages have no locale catalog, but retain Japanese home.
        .or_else(|| (locale == Locale::Ja && locales.routes.is_empty()).then(|| "/".into()))
}

pub(crate) fn choice_href(path: &str, locale: Locale) -> String {
    format!("{path}?lang={locale}")
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

pub(crate) async fn redirect(cx: &Cx) -> Option<Response> {
    let uri = request::uri(cx);
    let explicit = explicit_choice(uri);
    let locale = explicit
        .or_else(|| {
            cookies(cx)
                .get(COOKIE)
                .and_then(|cookie| cookie.value().parse().ok())
        })
        .unwrap_or_else(|| preferred_locale(cx));
    if explicit.is_none() && locale == Locale::Ja {
        return None;
    }
    let locales = match crate::app::page_loader(cx).load_locales().await {
        Ok(locales) => locales,
        Err(error) => {
            tracing::error!(%error, "language selection could not read the published locales");
            return None;
        }
    };
    let mut location = destination(&locales, uri.path(), locale)?;
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
    Ok(view! {
        toggle_group(
            attrs: attributes! {
                role="group"
                aria-label=(t(locale, Message::NavLanguage))
                class="shrink-0 border-primary/30 bg-background/45 shadow-[inset_0_1px_0_rgb(255_255_255/0.04)]"
            },
            #[key(other.as_str())]
            for other in Locale::ALL {
                if let Some(target) = destination(locales, path, other) {
                    toggle_link(
                        active: other == locale,
                        size: ToggleSize::Sm,
                        attrs: attributes! {
                            (link_attrs(
                                cx,
                                choice_href(&target, other),
                                PrefetchMode::Never,
                            ))
                            lang=(other.as_str())
                            aria-current=(if other == locale {
                                Some("true")
                            } else {
                                None
                            })
                        },
                        (if other == Locale::Ja { "日本語" } else { "English" })
                    )
                } else {
                    <span
                        lang=(other.as_str())
                        aria-disabled="true"
                        class="inline-flex h-8 shrink-0 items-center justify-center rounded-md border border-transparent px-2 text-xs font-medium text-muted-foreground opacity-50 select-none"
                    >
                        (if other == Locale::Ja { "日本語" } else { "English" })
                    </span>
                }
            }
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_ranges_require_valid_subtags_and_keep_the_first_highest_weight() {
        for (value, expected) in [
            ("*-US;q=1, en;q=0", Locale::En),
            ("ja-abcdefghi, en;q=0.5", Locale::En),
            ("ja-_, en;q=0.5", Locale::En),
            ("ja--JP, en;q=0.5", Locale::En),
            ("ja-abcdefgh, en;q=0.5", Locale::Ja),
            ("ja;q=0.4, en;q=0.6, ja;q=0.8", Locale::Ja),
            ("ja;q=0.8, en;q=0.6, ja;q=0.4", Locale::Ja),
            ("ja;q=0.8, en;q=0.8, ja;q=0.8", Locale::Ja),
            ("ja;q=0", Locale::En),
        ] {
            let (parts, ()) = request::Request::builder()
                .header(header::ACCEPT_LANGUAGE, value)
                .body(())
                .unwrap()
                .into_parts();
            assert_eq!(
                preferred_locale(&Cx::new().with(parts)),
                expected,
                "{value}"
            );
        }
    }

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
            Some("/tech/japanese-only")
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
