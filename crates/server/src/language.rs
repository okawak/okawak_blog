//! Browser language preference at the site's entry point; page URLs keep their locale.

use domain::{Locale, SiteLocalesDocument};
use topcoat::{
    context::Cx,
    router::{Body, HeaderMap, HeaderValue, StatusCode, Uri, header, request, response::Response},
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

fn preferred_locale(headers: &HeaderMap) -> Locale {
    for value in headers.get_all(header::COOKIE) {
        for cookie in value.to_str().unwrap_or_default().split(';') {
            if let Some((name, value)) = cookie.trim().split_once('=')
                && name == COOKIE
                && let Ok(locale) = value.parse()
            {
                return locale;
            }
        }
    }
    let mut japanese: Option<(u16, usize)> = None;
    let mut english: Option<(u16, usize)> = None;
    let mut wildcard: Option<(u16, usize)> = None;
    let ranges = headers
        .get_all(header::ACCEPT_LANGUAGE)
        .iter()
        .flat_map(|value| value.to_str().unwrap_or_default().split(','));
    for (order, range) in ranges.enumerate() {
        let mut parts = range.trim().split(';');
        let tag = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
        let slot = match tag.split('-').next() {
            Some("ja") => &mut japanese,
            Some("en") => &mut english,
            Some("*") if tag == "*" => &mut wildcard,
            _ => continue,
        };
        if tag != "*"
            && !tag.split('-').all(|part| {
                !part.is_empty()
                    && part.len() <= 8
                    && part.bytes().all(|c| c.is_ascii_alphanumeric())
            })
        {
            continue;
        }
        let weight = match (parts.next(), parts.next()) {
            (None, None) => Some(1000),
            (Some(parameter), None) => parameter.trim().strip_prefix("q=").and_then(quality),
            _ => None,
        };
        if let Some(weight) = weight
            && slot.is_none_or(|(previous, _)| weight > previous)
        {
            *slot = Some((weight, order));
        }
    }
    // Keep explicit zero weights: the wildcard must not re-enable an excluded language.
    [
        (Locale::En, english.or(wildcard)),
        (Locale::Ja, japanese.or(wildcard)),
    ]
    .into_iter()
    .filter_map(|(locale, weight)| {
        weight
            .filter(|(weight, _)| *weight > 0)
            .map(|(weight, order)| (locale, weight, order))
    })
    .min_by_key(|(_, weight, order)| (std::cmp::Reverse(*weight), *order))
    .map(|(locale, _, _)| locale)
    .unwrap_or(Locale::En)
}

fn quality(value: &str) -> Option<u16> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if !matches!(whole, "0" | "1")
        || fraction.len() > 3
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let fraction: u16 = format!("{fraction:0<3}").parse().ok()?;
    match whole {
        "0" => Some(fraction),
        "1" if fraction == 0 => Some(1000),
        _ => None,
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
    let locale = explicit.unwrap_or_else(|| preferred_locale(request::headers(cx)));
    if explicit.is_none() && locale == Locale::Ja {
        return None;
    }
    let locales = match crate::app::page_loader(cx).loader().load_locales().await {
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
    let mut response = Response::new(Body::empty());
    *response.status_mut() = if explicit.is_some() {
        StatusCode::SEE_OTHER
    } else {
        StatusCode::TEMPORARY_REDIRECT
    };
    response
        .headers_mut()
        .insert(header::LOCATION, HeaderValue::from_str(&location).ok()?);
    response.headers_mut().insert(
        header::VARY,
        HeaderValue::from_static("Accept-Language, Cookie"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if explicit.is_some() {
        response.headers_mut().insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&format!(
                "{COOKIE}={locale}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Lax"
            ))
            .ok()?,
        );
    }
    Some(response)
}

#[component]
pub(crate) async fn language_switcher(
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
            for other in Locale::ALL {
                if let Some(target) = destination(locales, path, other) {
                    toggle_link(
                        active: other == locale,
                        size: ToggleSize::Sm,
                        attrs: attributes! {
                            href=(choice_href(&target, other))
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
    fn qualities_are_bounded_http_weights() {
        for (input, expected) in [
            ("1", Some(1000)),
            ("1.000", Some(1000)),
            ("0.7", Some(700)),
            ("0.025", Some(25)),
            ("0", Some(0)),
            ("1.1", None),
            ("0.1234", None),
            ("NaN", None),
            ("-1", None),
        ] {
            assert_eq!(quality(input), expected, "{input}");
        }
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
