use crate::{DomainError, Result};

/// Hash-named public image. Safe as one URL and storage-key segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAssetName(String);

impl ContentAssetName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let valid = name.split_once('.').is_some_and(|(hash, extension)| {
            hash.len() == 64
                && hash.bytes().all(|b| b.is_ascii_hexdigit())
                && media_type(extension).is_some()
        });
        if !valid {
            return Err(DomainError::validation("invalid public asset name"));
        }
        Ok(Self(name))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn media_type(&self) -> &'static str {
        media_type(self.0.rsplit('.').next().unwrap()).expect("validated extension")
    }
}

fn media_type(extension: &str) -> Option<&'static str> {
    match extension {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_asset_keys_and_media_types() {
        assert_eq!(
            ContentAssetName::new(format!("{}.png", "a".repeat(64)))
                .unwrap()
                .media_type(),
            "image/png"
        );
        for bad in ["../secret.png", "image.png", "/file.jpg", "file.svg"] {
            assert!(ContentAssetName::new(bad).is_err());
        }
    }
}
