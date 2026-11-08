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
        for (extension, media_type) in [
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("gif", "image/gif"),
            ("webp", "image/webp"),
            ("avif", "image/avif"),
        ] {
            let name = format!("{}.{extension}", "a".repeat(64));
            let asset = ContentAssetName::new(name.clone()).unwrap();
            assert_eq!(asset.as_str(), name);
            assert_eq!(asset.media_type(), media_type);
        }
        for bad in ["../secret.png", "image.png", "/file.jpg", "file.svg"] {
            assert!(ContentAssetName::new(bad).is_err());
        }
    }
}
