use leptos::*;
use leptos_router::*;

#[component]
pub fn CategoryPage() -> impl IntoView {
    // Creates a reactive value to update the button
    let count = RwSignal::new(0);
    let on_click = move |_| *count.write() += 1;

    view! {
        <h1>"開発中"</h1>
        <h1 class="text-3xl font-bold underline">tailwind CSSのテスト</h1>
        <button on:click=on_click>"ボタン要素: " {count}</button>
    }
    //// URLパラメータからカテゴリ名を取得
    //let params = use_params_map(cx);
    //let category = move || params.with(|p| p.get("category").cloned().unwrap_or_default());

    //// 仮のデータ：カテゴリに対応する記事一覧
    //let articles = move || match category().as_str() {
    //    "tech" => vec!["rust", "webassembly", "leptos"],
    //    "physics" => vec!["quantum", "relativity"],
    //    "statistics" => vec!["bayesian", "likelihood"],
    //    "daily" => vec!["diary1", "diary2"],
    //    _ => vec![],
    //};

    //view! {
    //    <h1>{format!("Category: {}", category())}</h1>
    //    <ul>
    //        {articles()
    //            .into_iter()
    //            .map(|id| {
    //                view! { cx,
    //                    <li>
    //                        <A href=format!("/{}/{}", category(), id)>{id}</A>
    //                    </li>
    //                }
    //            })
    //            .collect::<Vec<_>>()}
    //    </ul>
    //}
}
