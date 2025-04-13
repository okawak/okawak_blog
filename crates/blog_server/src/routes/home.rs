use crate::components::header::Header;
use leptos::*;
use leptos_router::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <Header />
        <main class="p-4">
            <ul class="space-y-2">
                <li>
                    <A href="/daily">"日常"</A>
                </li>
                <li>
                    <A href="/tech">"技術"</A>
                </li>
                <li>
                    <A href="/statistics">"統計"</A>
                </li>
            </ul>
        </main>
    }
}
