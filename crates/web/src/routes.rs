// モダンなRustスタイル - サブモジュールを宣言
pub mod about;
pub mod home;
pub mod not_found;

// 必要に応じてサブモジュールからコンポーネントを再エクスポート
pub use about::AboutPage;
pub use home::HomePage;
pub use not_found::NotFoundPage;
