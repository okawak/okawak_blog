use crate::components::{NavigationItem, get_main_nav_items};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use stylance::import_style;
use thaw::*;

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

                // ハンバーガーボタン（thaw-ui Buttonで置き換え）
                <Button
                    class=header_style::menu_toggle
                    on_click=move |_| set_menu_open.update(|v| *v = !*v)
                >
                    <div class=header_style::hamburger_icon>
                        <span class=header_style::bar></span>
                        <span class=header_style::bar></span>
                        <span class=header_style::bar></span>
                    </div>
                </Button>

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

                    // ソーシャルリンクにthaw-ui Buttonを使用
                    <div class=header_style::social_links>
                        <Button
                            class=header_style::social_button
                            on_click=move |_| {
                                if let Some(window) = leptos::web_sys::window() {
                                    let _ = window.open_with_url_and_target("https://github.com/okawak", "_blank");
                                }
                            }
                        >
                            <i class="fab fa-github"></i>
                        </Button>
                    </div>
                </nav>
            </div>
        </header>
    }
}
