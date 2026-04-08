use leptos::{oco::Oco, prelude::*, text_prop::TextProp};
use leptos_meta::{Link, Meta, Title};

#[component]
pub fn PageMetadata(
    #[prop(into)] title: TextProp,
    #[prop(into)] description: TextProp,
    #[prop(into)] canonical_url: Oco<'static, str>,
    #[prop(optional, into)] og_type: Option<TextProp>,
) -> impl IntoView {
    let og_type = og_type.unwrap_or_else(|| "website".into());

    view! {
        <Title text=title.clone() />
        <Meta name="description" content=description.clone() />
        <Link rel="canonical" href=canonical_url.clone() />
        <Meta property="og:title" content=title />
        <Meta property="og:description" content=description />
        <Meta property="og:url" content=canonical_url />
        <Meta property="og:type" content=og_type />
    }
}
