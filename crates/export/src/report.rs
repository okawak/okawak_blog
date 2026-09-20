use domain::Slug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedContent {
    Article(Slug),
    Tag(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationReport<T> {
    pub generated: usize,
    pub reused: usize,
    pub protected: Vec<T>,
}

impl<T> Default for TranslationReport<T> {
    fn default() -> Self {
        Self {
            generated: 0,
            reused: 0,
            protected: Vec::new(),
        }
    }
}
