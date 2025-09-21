//! Authentication Extractors - 認証用Extractors（スタブ）

/// 認証済みユーザー情報
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
}

/// 管理者権限チェック用Extractor
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);
