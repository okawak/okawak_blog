use crate::routes::not_found::NotFoundPage;
use domain::CategoryPageDocument;
#[cfg(feature = "ssr")]
use domain::build_category_page_document;
#[cfg(feature = "ssr")]
use infra::DynArtifactReader;
use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
use stylance::import_style;

import_style!(category_style, "category.module.scss");

#[server]
pub async fn get_category_page_document(
    category: String,
) -> Result<Option<CategoryPageDocument>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let artifact_reader = use_context::<DynArtifactReader>()
            .ok_or_else(|| ServerFnError::new("artifact reader context is missing"))?;
        if category.trim().is_empty() {
            return Ok(None);
        }

        let category_index = match artifact_reader.read_category_index(&category).await {
            Ok(index) => index,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        Ok(Some(build_category_page_document(&category_index)?))
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = category;
        Err(ServerFnError::new(
            "get_category_page_document is only available during SSR",
        ))
    }
}

#[component]
fn CategoryPageContent(document: CategoryPageDocument) -> impl IntoView {
    let title = document.category_display_name;
    let description = format!("{} カテゴリの記事一覧です。", title);
    let articles = document.articles;

    view! {
        <div class=category_style::category_page>
            <header class=category_style::category_header>
                <p class=category_style::eyebrow>{"Category"}</p>
                <h1 class=category_style::category_title>{title}</h1>
                <p class=category_style::category_description>{description}</p>
            </header>

            <section class=category_style::article_list>
                <For
                    each=move || articles.clone()
                    key=|article| article.slug.as_str().to_string()
                    children=move |article| {
                        let href = format!("/articles/{}", article.slug.as_str());
                        let title = article.title.as_str().to_string();
                        let description = article
                            .description
                            .unwrap_or_else(|| "説明はまだありません。".to_string());
                        let updated_at = article.updated_at;

                        view! {
                            <article class=category_style::article_card>
                                <h2 class=category_style::article_title>
                                    <a class=category_style::article_link href=href>
                                        {title}
                                    </a>
                                </h2>
                                <p class=category_style::article_description>{description}</p>
                                <p class=category_style::article_meta>
                                    {format!("更新 {}", updated_at)}
                                </p>
                            </article>
                        }
                    }
                />
            </section>
        </div>
    }
}

#[component]
pub fn CategoryPage() -> impl IntoView {
    let params = use_params_map();
    let category =
        move || params.with(|params: &ParamsMap| params.get("category").unwrap_or_default());
    let category_page = Resource::<Result<Option<CategoryPageDocument>, String>>::new(
        category,
        move |category| async move {
            if category.is_empty() {
                return Ok(None);
            }

            get_category_page_document(category)
                .await
                .map_err(|error| error.to_string())
        },
    );

    view! {
        <Suspense fallback=|| {
            view! { <div class=category_style::loading>"カテゴリを読み込み中..."</div> }
        }>
            {move || match category_page.get() {
                Some(Ok(Some(document))) => view! { <CategoryPageContent document /> }.into_any(),
                Some(Ok(None)) => view! { <NotFoundPage /> }.into_any(),
                Some(Err(error)) => view! {
                    <div class=category_style::error>
                        {format!("カテゴリの読み込みに失敗しました: {error}")}
                    </div>
                }
                .into_any(),
                None => view! { <div class=category_style::loading></div> }.into_any(),
            }}
        </Suspense>
    }
}
