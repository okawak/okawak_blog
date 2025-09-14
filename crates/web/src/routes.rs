// サブモジュールを公開
//pub mod article;
//pub mod category;
pub mod home;
pub mod not_found;

// 必要に応じてサブモジュールからコンポーネントを再エクスポート
//pub use article::ArticlePage;
//pub use category::CategoryPage;
pub use home::HomePage;
pub use not_found::NotFoundPage;
