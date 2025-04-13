use leptos::*;
use leptos_router::*;
use crate::content::{parser::Article, s3};

#[server]
async fn fetch_article(cat: String, slug: String) -> Result<Article, ServerFnError> {
    let key = format!("{}/{}.md", cat, slug);
    s3::get(&key).await?.ok_or(ServerFnError::ServerError("not found".into()))
}

#[component]
pub fn ArticlePage() -> impl IntoView {
    let params = use_params_map();
    let cat  = move || params.with(|m| m.get("cat").cloned().unwrap());
    let slug = move || params.with(|m| m.get("slug").cloned().unwrap());

    let art = create_resource(
        move || (cat(), slug()),
        |(c, s)| async move { fetch_article(c, s).await }
    );

    view! {
        <Transition fallback=|| view!"Loading…">
            {move || art.get().map(|a| view! {
                <article class="prose mx-auto p-4">
                    <h1 class="mb-2">{a.fm.title}</h1>
                    <p class="text-sm text-gray-500">{a.fm.date.to_string()}</p>
                    // XSS 対策するなら ammonia などで sanitize
                    <div inner_html=a.html></div>
                </article>
            })}
        </Transition>
    }
}
