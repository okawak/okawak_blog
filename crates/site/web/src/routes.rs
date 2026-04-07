// Declare route submodules.
pub mod about;
pub mod article;
pub mod category;
pub mod home;
pub mod not_found;

// Re-export route components for convenient access.
pub use about::AboutPage;
pub use article::ArticlePage;
pub use category::CategoryPage;
pub use home::HomePage;
pub use not_found::NotFoundPage;
