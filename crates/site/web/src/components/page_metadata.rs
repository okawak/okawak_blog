use leptos::prelude::*;
use leptos_meta::{Meta, Title};

#[component]
pub fn PageMetadata(title: String, description: String) -> impl IntoView {
    view! {
        <Title text=title />
        <Meta name="description" content=description />
    }
}
