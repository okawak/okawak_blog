use crate::components::{NavigationItem, get_main_nav_items};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use stylance::import_style;

import_style!(header_style, "header.module.scss");

/// サイトヘッダーコンポーネント
#[component]
pub fn Header() -> impl IntoView {
    let location = use_location();
    let nav_items = Memo::new(move |_| get_main_nav_items(&location.pathname.get()));

    let (menu_open, set_menu_open) = signal(false);

    view! {
        <header class=header_style::header class:open=move || menu_open.get()>
            <div class=header_style::container>
                <a href="/">
                    <h1 class=header_style::logo>{"ぶくせんの探窟メモ"}</h1>
                </a>

                // ハンバーガーボタン
                <button
                    class=header_style::menu_toggle
                    on:click=move |_| set_menu_open.update(|v| *v = !*v)
                    aria-label="Toggle menu"
                >
                    <span class=header_style::bar></span>
                    <span class=header_style::bar></span>
                    <span class=header_style::bar></span>
                </button>

                <nav class=move || {
                    let state = if menu_open.get() {
                        header_style::open
                    } else {
                        header_style::closed
                    };
                    format!("{} {}", header_style::nav_container, state)
                }>
                    <ul class=header_style::nav_list>
                        <For
                            each=move || nav_items.get()
                            key=|item: &NavigationItem| item.href.clone()
                            children=move |child| {
                                let href = child.href.clone();
                                let active_class = move || {
                                    if location.pathname.get() == href {
                                        header_style::nav_item_active
                                    } else {
                                        header_style::nav_item
                                    }
                                };
                                view! {
                                    <li class=active_class>
                                        <a class=header_style::nav_link href=child.href>
                                            {child.title}
                                        </a>
                                    </li>
                                }
                            }
                        />
                    </ul>
                    <div class=header_style::social_links>
                        <a
                            href="https://github.com/okawak"
                            class=header_style::social_icon
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            <i class="fab fa-github"></i>
                        </a>
                    </div>
                </nav>
            </div>
        </header>
    }
}
