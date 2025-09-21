//! Leptos Integration - Leptos Server Functions との統合（将来実装）

use crate::usecases::BlogUseCases;
use std::sync::Arc;

/// Server Functions 用のコンテキスト（将来実装）
#[derive(Clone)]
pub struct ServerContext<R, S> {
    pub use_cases: Arc<BlogUseCases<R, S>>,
}

impl<R, S> ServerContext<R, S> {
    pub fn new(use_cases: Arc<BlogUseCases<R, S>>) -> Self {
        Self { use_cases }
    }
}

// Leptos Server Functions は将来実装予定
// 現在はAxum REST APIのみを提供
