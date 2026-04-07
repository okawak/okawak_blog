//! Stub authentication extractors.

/// Authenticated user information.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
}

/// Extractor marker for administrator access.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);
