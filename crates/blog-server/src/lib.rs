#![warn(unreachable_pub)]

//! Topcoat site application, runtime composition, and HTTP boundary.

mod article_card;
mod article_filter;
mod artifact_page_loader;
mod category_articles;

// Topcoat UI vendors public component APIs into an application-private module.
#[allow(unreachable_pub)]
mod components;

pub mod app;

pub mod assets;

mod format;

mod icons;

pub mod http_cache;

mod page_loader;

mod shell;

mod i18n;
mod language;
mod metadata;
