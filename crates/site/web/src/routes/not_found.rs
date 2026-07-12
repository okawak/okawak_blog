use crate::components::PageMetadata;
use crate::{SITE_NAME, build_site_url};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

const NOT_FOUND_TITLE: &str = "ページが見つかりません";
const NOT_FOUND_DESCRIPTION: &str = "お探しのページは見つかりませんでした。";

/// 404 page component.
/// Rendered when the requested URL does not exist.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    let location = use_location();
    let canonical_url = build_site_url(&location.pathname.get());

    view! {
        <PageMetadata
            title=format!("{NOT_FOUND_TITLE} | {SITE_NAME}")
            description=NOT_FOUND_DESCRIPTION
            canonical_url
        />
        <div>{"ページが見つかりませんでした。"}</div>
    }
}
