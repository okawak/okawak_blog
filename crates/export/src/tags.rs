//! Aggregate only public tag IDs; labels are translated once per unique tag.
use crate::Result;
use domain::{LabelCatalog, LabelEntry, Locale};
use std::{collections::BTreeSet, fs, path::Path};

pub(crate) fn sync(stage: &Path) -> Result<()> {
    let path = stage.join("tags.json");
    let mut catalog = if path.exists() {
        crate::catalog::read(&path)?
    } else {
        LabelCatalog::default()
    };
    let tags = crate::markdown::read_locale(stage, Locale::Ja)?
        .into_iter()
        .flat_map(|d| d.meta.tags)
        .collect::<BTreeSet<_>>();
    if catalog.entries.keys().any(|key| !tags.contains(key)) {
        let bytes = fs::read(&path)?;
        let archive = stage
            .join(".export-archive")
            .join(format!("tags-{}.json", crate::vault::digest(&bytes)));
        fs::create_dir_all(archive.parent().unwrap())?;
        fs::write(archive, bytes)?;
    }
    catalog.entries.retain(|key, _| tags.contains(key));
    for tag in tags {
        catalog.entries.entry(tag.clone()).or_insert(LabelEntry {
            source: tag,
            context:
                "Public blog tag label; concise noun phrase. Do not change technical proper nouns."
                    .into(),
            translation: None,
        });
    }
    catalog.validate()?;
    crate::catalog::write(&path, &catalog)
}
