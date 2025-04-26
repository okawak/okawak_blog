use crate::components::{get_main_nav_items, NavigationItem};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use reactive_stores::Store;
use stylance::import_style;

import_style!(header_style, "./header.module.scss");

#[derive(Store, Clone)]
pub struct NavItemsStore {
    #[store(key: String = |item| item.href.clone())]
    items: Vec<NavigationItem>,
}

/// サイトヘッダーコンポーネント
#[component]
pub fn Header() -> impl IntoView {
    let location = use_location();
    let current = move || location.pathname.get();
    let nav_store = Store::new(NavItemsStore {
        items: get_main_nav_items(&current()),
    });

    view! {
        <header class=header_style::header>
            <div class=header_style::container>
                <a href="/">
                    <h1 class=header_style::logo>{"ぶくせんの探窟メモ"}</h1>
                </a>

                <nav>
                    <ul class=header_style::nav_list>
                        <For
                            each=move || nav_store.items()
                            key=|child| child.read().href.clone()
                            children=move |child| {
                                let nav = child.read();
                                let href = nav.href.clone();
                                let title = nav.title.clone();
                                let active_class = if nav.is_active {
                                    header_style::nav_item_active
                                } else {
                                    header_style::nav_item
                                };
                                view! {
                                    <li class=active_class>
                                        <a class=header_style::nav_link href=href>
                                            {title}
                                        </a>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </nav>

                <div class=header_style::social_links>
                    <a href="https://github.com/okawak" class=header_style::social_icon>
                        <i class="fab fa-github"></i>
                    </a>
                </div>
            </div>
        </header>
    }
}
