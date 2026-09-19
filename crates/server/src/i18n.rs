//! Server-owned message keys, interpolation, and fallback policy.
use domain::{Category, LabelCatalog, Locale};
use std::sync::LazyLock;

static CATALOG: LazyLock<LabelCatalog> = LazyLock::new(|| {
    let catalog: LabelCatalog = serde_json::from_str(include_str!("../locales/ui.json"))
        .expect("invalid bundled UI catalog");
    catalog.validate().expect("invalid bundled UI translations");
    catalog
});

#[derive(Clone, Copy, Debug)]
pub(crate) enum Message {
    ArticleNoDescription,
    ArticlePublished,
    ArticleTags,
    ArticleUpdated,
    CategoryArticles,
    CategoryDaily,
    CategoryGeneral,
    CategoryLabel,
    CategoryPhysics,
    CategoryStatistics,
    CategoryTech,
    CountArticlesMany,
    CountArticlesOne,
    CountArticlesZero,
    CountCategoriesMany,
    CountCategoriesOne,
    CountCategoriesZero,
    EmptyArticles,
    EmptyMatches,
    ErrorArticle,
    ErrorCategory,
    ErrorHome,
    ErrorNotFoundDescription,
    ErrorNotFoundBody,
    ErrorNotFoundTitle,
    ErrorPage,
    FilterLabel,
    FilterPlaceholder,
    FooterCopyright,
    FooterPowered,
    HomeDescription,
    HomeEyebrow,
    HomeFallback,
    HomeIntro,
    HomeRecent,
    MetaArticle,
    MetaCategory,
    MetaHome,
    MetaPage,
    NavAbout,
    NavClose,
    NavGithub,
    NavHome,
    NavLanguage,
    NavMain,
    NavOpen,
    PageLabel,
    SiteName,
}
impl Message {
    fn key(self) -> &'static str {
        match self {
            Self::ArticleNoDescription => "article.no_description",
            Self::ArticlePublished => "article.published",
            Self::ArticleTags => "article.tags",
            Self::ArticleUpdated => "article.updated",
            Self::CategoryArticles => "category.articles",
            Self::CategoryDaily => "category.daily",
            Self::CategoryGeneral => "category.general",
            Self::CategoryLabel => "category.label",
            Self::CategoryPhysics => "category.physics",
            Self::CategoryStatistics => "category.statistics",
            Self::CategoryTech => "category.tech",
            Self::CountArticlesMany => "count.articles.many",
            Self::CountArticlesOne => "count.articles.one",
            Self::CountArticlesZero => "count.articles.zero",
            Self::CountCategoriesMany => "count.categories.many",
            Self::CountCategoriesOne => "count.categories.one",
            Self::CountCategoriesZero => "count.categories.zero",
            Self::EmptyArticles => "empty.articles",
            Self::EmptyMatches => "empty.matches",
            Self::ErrorArticle => "error.article",
            Self::ErrorCategory => "error.category",
            Self::ErrorHome => "error.home",
            Self::ErrorNotFoundDescription => "error.not_found.description",
            Self::ErrorNotFoundBody => "error.not_found.message",
            Self::ErrorNotFoundTitle => "error.not_found.title",
            Self::ErrorPage => "error.page",
            Self::FilterLabel => "filter.label",
            Self::FilterPlaceholder => "filter.placeholder",
            Self::FooterCopyright => "footer.copyright",
            Self::FooterPowered => "footer.powered",
            Self::HomeDescription => "home.description",
            Self::HomeEyebrow => "home.eyebrow",
            Self::HomeFallback => "home.fallback",
            Self::HomeIntro => "home.intro",
            Self::HomeRecent => "home.recent",
            Self::MetaArticle => "meta.article",
            Self::MetaCategory => "meta.category",
            Self::MetaHome => "meta.home",
            Self::MetaPage => "meta.page",
            Self::NavAbout => "nav.about",
            Self::NavClose => "nav.close",
            Self::NavGithub => "nav.github",
            Self::NavHome => "nav.home",
            Self::NavLanguage => "nav.language",
            Self::NavMain => "nav.main",
            Self::NavOpen => "nav.open",
            Self::PageLabel => "page.label",
            Self::SiteName => "site.name",
        }
    }
}

pub(crate) fn t(locale: Locale, message: Message) -> &'static str {
    lookup(&CATALOG, locale, message.key())
}
fn lookup<'a>(catalog: &'a LabelCatalog, locale: Locale, key: &'a str) -> &'a str {
    let Some(entry) = catalog.entries.get(key) else {
        tracing::error!(key, "missing UI message key");
        return key;
    };
    if locale == Locale::En && entry.translation.as_ref().is_none_or(|t| t.stale) {
        tracing::warn!(key, "English UI message unavailable; using Japanese");
    }
    entry.value(locale)
}

pub(crate) fn interpolate(locale: Locale, message: Message, values: &[(&str, String)]) -> String {
    let template = t(locale, message);
    let mut result = String::new();
    let mut rest = template;
    while let Some((before, after)) = rest.split_once('{') {
        result.push_str(before);
        let Some((key, tail)) = after.split_once('}') else {
            result.push('{');
            result.push_str(after);
            return result;
        };
        if let Some((_, value)) = values.iter().find(|(name, _)| *name == key) {
            result.push_str(value);
        } else {
            tracing::error!(key, "missing UI interpolation value");
            result.push('{');
            result.push_str(key);
            result.push('}');
        }
        rest = tail;
    }
    result.push_str(rest);
    result
}

pub(crate) fn article_count(locale: Locale, count: usize) -> String {
    interpolate(
        locale,
        match count {
            0 => Message::CountArticlesZero,
            1 => Message::CountArticlesOne,
            _ => Message::CountArticlesMany,
        },
        &[("count", count.to_string())],
    )
}
pub(crate) fn category_count(locale: Locale, count: usize) -> String {
    interpolate(
        locale,
        match count {
            0 => Message::CountCategoriesZero,
            1 => Message::CountCategoriesOne,
            _ => Message::CountCategoriesMany,
        },
        &[("count", count.to_string())],
    )
}
pub(crate) fn category_name(locale: Locale, category: Category) -> &'static str {
    t(
        locale,
        match category {
            Category::Tech => Message::CategoryTech,
            Category::Daily => Message::CategoryDaily,
            Category::Physics => Message::CategoryPhysics,
            Category::Statistics => Message::CategoryStatistics,
        },
    )
}
pub(crate) fn locale_from_path(path: &str) -> Locale {
    if path == "/en" || path.starts_with("/en/") {
        Locale::En
    } else {
        Locale::Ja
    }
}
pub(crate) fn japanese_path(path: &str) -> &str {
    if path == "/en" || path == "/en/" {
        "/"
    } else {
        path.strip_prefix("/en/")
            .map(|_| &path[3..])
            .unwrap_or(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_message_key_has_both_languages() {
        let keys = [
            Message::ArticleNoDescription,
            Message::ArticlePublished,
            Message::ArticleTags,
            Message::ArticleUpdated,
            Message::CategoryArticles,
            Message::CategoryDaily,
            Message::CategoryGeneral,
            Message::CategoryLabel,
            Message::CategoryPhysics,
            Message::CategoryStatistics,
            Message::CategoryTech,
            Message::CountArticlesMany,
            Message::CountArticlesOne,
            Message::CountArticlesZero,
            Message::CountCategoriesMany,
            Message::CountCategoriesOne,
            Message::CountCategoriesZero,
            Message::EmptyArticles,
            Message::EmptyMatches,
            Message::ErrorArticle,
            Message::ErrorCategory,
            Message::ErrorHome,
            Message::ErrorNotFoundDescription,
            Message::ErrorNotFoundBody,
            Message::ErrorNotFoundTitle,
            Message::ErrorPage,
            Message::FilterLabel,
            Message::FilterPlaceholder,
            Message::FooterCopyright,
            Message::FooterPowered,
            Message::HomeDescription,
            Message::HomeEyebrow,
            Message::HomeFallback,
            Message::HomeIntro,
            Message::HomeRecent,
            Message::MetaArticle,
            Message::MetaCategory,
            Message::MetaHome,
            Message::MetaPage,
            Message::NavAbout,
            Message::NavClose,
            Message::NavGithub,
            Message::NavHome,
            Message::NavLanguage,
            Message::NavMain,
            Message::NavOpen,
            Message::PageLabel,
            Message::SiteName,
        ];
        assert_eq!(
            keys.len(),
            CATALOG.entries.len(),
            "new entries require a typed key"
        );
        for key in keys {
            let entry = CATALOG
                .entries
                .get(key.key())
                .unwrap_or_else(|| panic!("missing {}", key.key()));
            assert!(
                entry.translation.as_ref().is_some_and(|t| !t.stale),
                "missing or stale {}",
                key.key()
            );
        }
    }
    #[test]
    fn counts_fallback_and_interpolation_are_explicit() {
        assert_eq!(article_count(Locale::En, 0), "0 articles");
        assert_eq!(article_count(Locale::En, 1), "1 article");
        assert_eq!(article_count(Locale::En, 2), "2 articles");
        assert_eq!(article_count(Locale::Ja, 1), "1件の記事");
        assert_eq!(
            lookup(&LabelCatalog::default(), Locale::En, "missing"),
            "missing"
        );
        let mut catalog = CATALOG.clone();
        catalog.entries.get_mut("nav.home").unwrap().translation = None;
        assert_eq!(lookup(&catalog, Locale::En, "nav.home"), "ホーム");
        assert_eq!(
            interpolate(
                Locale::En,
                Message::MetaPage,
                &[("title", "{category}".into())]
            ),
            "The {category} page."
        );
        assert_eq!(locale_from_path("/english"), Locale::Ja);
        assert_eq!(locale_from_path("/en/tech"), Locale::En);
        assert_eq!(japanese_path("/en/tech"), "/tech");
    }
}
