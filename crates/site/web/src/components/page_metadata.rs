use leptos::{prelude::*, text_prop::TextProp};
use leptos_meta::{Meta, Title};

#[component]
pub fn PageMetadata(
    #[prop(into)] title: TextProp,
    #[prop(into)] description: TextProp,
) -> impl IntoView {
    view! {
        <Title text=title />
        <Meta name="description" content=description />
    }
}
