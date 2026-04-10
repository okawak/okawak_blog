use crate::SITE_NAME;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::{NavigationItem, get_main_nav_items};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;
use stylance::import_style;

import_style!(header_style, "header.module.scss");

/// Site header component.
#[component]
pub fn Header() -> impl IntoView {
    const NAV_ID: &str = "site-header-nav";

    let location = use_location();
    let nav_items = Memo::new(move |_| get_main_nav_items(&location.pathname.get()));

    let (menu_open, set_menu_open) = signal(false);

    view! {
        <header class=header_style::header class:open=move || menu_open.get()>
            <div class=header_style::container>
                <A href="/">
                    <h1 class=header_style::logo>{SITE_NAME}</h1>
                </A>

                <Button
                    class=header_style::menu_toggle
                    size=ButtonSize::Icon
                    variant=ButtonVariant::Ghost
                    attr:aria-controls=NAV_ID
                    attr:aria-expanded=move || if menu_open.get() { "true" } else { "false" }
                    attr:aria-label="Toggle navigation menu"
                    on:click=move |_| set_menu_open.update(|v| *v = !*v)
                >
                    <div class=header_style::hamburger_icon>
                        <span class=header_style::bar></span>
                        <span class=header_style::bar></span>
                        <span class=header_style::bar></span>
                    </div>
                </Button>

                <nav
                    id=NAV_ID
                    class=move || {
                        let state = if menu_open.get() {
                            header_style::open
                        } else {
                            header_style::closed
                        };
                        format!("{} {}", header_style::nav_container, state)
                    }
                >
                    <ul class=header_style::nav_list>
                        <For
                            each=move || nav_items.get()
                            key=|item: &NavigationItem| item.href.clone()
                            children=move |child| {
                                let href = child.href.clone();
                                let active_href = href.clone();
                                let active_class = move || {
                                    if location.pathname.get() == active_href {
                                        header_style::nav_item_active
                                    } else {
                                        header_style::nav_item
                                    }
                                };
                                view! {
                                    <li class=active_class>
                                        <A
                                            href=move || href.clone()
                                            {..}
                                            class=header_style::nav_link
                                            on:click=move |_| set_menu_open.set(false)
                                        >
                                            {child.title}
                                        </A>
                                    </li>
                                }
                            }
                        />
                    </ul>

                    <div class=header_style::social_links>
                        <Button
                            href="https://github.com/okawak"
                            class=header_style::social_button
                            size=ButtonSize::Icon
                            variant=ButtonVariant::Ghost
                            attr:aria-label="Open okawak GitHub profile"
                            attr:rel="noopener noreferrer"
                            attr:target="_blank"
                        >
                            <i class="fab fa-github" aria-hidden="true"></i>
                        </Button>
                    </div>
                </nav>
            </div>
        </header>
    }
}
