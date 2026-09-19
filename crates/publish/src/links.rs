use crate::{PublishError, Result, input::Document};
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn assets(documents: &[Document]) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for doc in documents {
        for event in Parser::new_ext(&doc.body, Options::ENABLE_TABLES) {
            if let Event::Start(Tag::Image { dest_url, .. } | Tag::Link { dest_url, .. }) = event
                && let Some(name) = dest_url.strip_prefix("/content-assets/")
            {
                domain::ContentAssetName::new(name)?;
                names.insert(name.to_owned());
            }
        }
    }
    Ok(names)
}

#[derive(Default)]
pub(crate) struct Index {
    routes: HashMap<String, String>,
}

impl Index {
    pub(crate) fn new(japanese: &[Document], localized: &[Document]) -> Self {
        let mut routes: HashMap<_, _> = japanese
            .iter()
            .map(|d| (d.meta.id.to_string(), d.meta.path()))
            .collect();
        routes.extend(
            localized
                .iter()
                .map(|d| (d.meta.id.to_string(), d.meta.path())),
        );
        Self { routes }
    }

    fn resolve(&self, target: &str) -> Option<String> {
        let rest = target.strip_prefix("content:")?;
        let (id, anchor) = rest
            .split_once('#')
            .map(|(id, a)| (id, format!("#{a}")))
            .unwrap_or((rest, String::new()));
        self.routes.get(id).map(|url| format!("{url}{anchor}"))
    }

    pub(crate) fn validate(&self, documents: &[Document]) -> Result<()> {
        for document in documents {
            for event in Parser::new_ext(
                &document.body,
                Options::ENABLE_WIKILINKS | Options::ENABLE_TABLES,
            ) {
                if let Event::Start(
                    Tag::Link {
                        link_type,
                        dest_url,
                        ..
                    }
                    | Tag::Image {
                        link_type,
                        dest_url,
                        ..
                    },
                ) = event
                {
                    if matches!(link_type, LinkType::WikiLink { .. }) {
                        return Err(PublishError::Parse(
                            "public Markdown must normalize WikiLinks with export".into(),
                        ));
                    }
                    let href = dest_url.as_ref();
                    if href.starts_with("content:") {
                        if self.resolve(href).is_none() {
                            return Err(PublishError::Parse(
                                "unresolved public content reference".into(),
                            ));
                        }
                    } else if !["https://", "http://", "mailto:", "#", "/content-assets/"]
                        .iter()
                        .any(|prefix| href.starts_with(prefix))
                    {
                        return Err(PublishError::Parse(
                            "public Markdown contains a non-public relative reference".into(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        Self {
            routes: HashMap::from([
                ("article".into(), "/tech/def456".into()),
                ("reference".into(), "/daily/ghi789".into()),
            ]),
        }
    }
}

pub(crate) fn resolve_links<'a>(
    events: impl Iterator<Item = Event<'a>> + 'a,
    index: &'a Index,
) -> impl Iterator<Item = Event<'a>> + 'a {
    events.map(move |event| match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: destination(dest_url, index),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: destination(dest_url, index),
            title,
            id,
        }),
        other => other,
    })
}

fn destination<'a>(target: CowStr<'a>, index: &Index) -> CowStr<'a> {
    index.resolve(&target).map(|s| s.into()).unwrap_or(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_public_identity_and_preserves_heading_anchor() {
        let index = Index::fixture();
        assert_eq!(
            index.resolve("content:article#section-a"),
            Some("/tech/def456#section-a".into())
        );
        assert_eq!(index.resolve("content:missing"), None);
    }
}
