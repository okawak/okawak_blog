use crate::SITE_NAME;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::{NavigationItem, get_main_nav_items};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

/// Site header component.
#[component]
pub fn Header() -> impl IntoView {
    const NAV_ID: &str = "site-header-nav";

    let location = use_location();
    let nav_items = Memo::new(move |_| get_main_nav_items(&location.pathname.get()));
    let (menu_open, set_menu_open) = signal(false);

    view! {
        <header class="sticky top-0 z-50 h-[var(--site-header-height)] border-b border-border/60 bg-background/95 shadow-[0_8px_24px_rgb(0_0_0/0.45)] backdrop-blur-sm">
            <div class="relative mx-auto flex h-full max-w-[var(--site-content-width)] items-center justify-between gap-3 px-4 sm:px-6">
                <A
                    href="/"
                    {..}
                    class="min-w-0 text-foreground no-underline transition-colors hover:text-primary focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                >
                    <h1 class="m-0 truncate text-xl leading-tight font-bold sm:text-2xl">
                        {SITE_NAME}
                    </h1>
                </A>

                <Button
                    class="shrink-0 md:hidden"
                    size=ButtonSize::Icon
                    variant=ButtonVariant::Ghost
                    attr:aria-controls=NAV_ID
                    attr:aria-expanded=move || if menu_open.get() { "true" } else { "false" }
                    attr:aria-label=move || {
                        if menu_open.get() {
                            "ナビゲーションメニューを閉じる"
                        } else {
                            "ナビゲーションメニューを開く"
                        }
                    }
                    on:click=move |_| set_menu_open.update(|open| *open = !*open)
                >
                    <div
                        class="flex size-5 flex-col items-center justify-center gap-1.5"
                        aria-hidden="true"
                    >
                        <span class=move || menu_bar_class(menu_open.get(), MenuBar::Top)></span>
                        <span class=move || menu_bar_class(menu_open.get(), MenuBar::Middle)></span>
                        <span class=move || menu_bar_class(menu_open.get(), MenuBar::Bottom)></span>
                    </div>
                </Button>

                <nav
                    id=NAV_ID
                    class=move || {
                        let mobile_state = if menu_open.get() { "flex" } else { "hidden" };
                        format!(
                            "{mobile_state} absolute inset-x-4 top-[calc(100%+0.5rem)] flex-col gap-3 rounded-lg border border-border bg-card/98 p-4 shadow-[0_18px_36px_rgb(0_0_0/0.55)] backdrop-blur-sm md:static md:flex md:flex-row md:items-center md:gap-6 md:border-0 md:bg-transparent md:p-0 md:shadow-none",
                        )
                    }
                >
                    <ul class="m-0 flex list-none flex-col gap-1 p-0 md:flex-row md:items-center md:gap-2">
                        <For
                            each=move || nav_items.get()
                            key=|item: &NavigationItem| item.href.clone()
                            children=move |child| {
                                let href = child.href.clone();
                                let active_href = href.clone();
                                let link_class = move || {
                                    if location.pathname.get() == active_href {
                                        "block rounded-md border-b-2 border-primary px-3 py-2 text-sm font-medium text-foreground no-underline"
                                    } else {
                                        "block rounded-md border-b-2 border-transparent px-3 py-2 text-sm font-medium text-muted-foreground no-underline transition-colors hover:border-primary hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                                    }
                                };
                                view! {
                                    <li>
                                        <A
                                            href={href}
                                            {..}
                                            class=link_class
                                            on:click=move |_| set_menu_open.set(false)
                                        >
                                            {child.title}
                                        </A>
                                    </li>
                                }
                            }
                        />
                    </ul>

                    <div class="border-t border-border pt-3 md:border-t-0 md:pt-0">
                        <Button
                            href="https://github.com/okawak"
                            class="text-foreground hover:text-primary"
                            size=ButtonSize::Icon
                            variant=ButtonVariant::Ghost
                            attr:aria-label="Open okawak GitHub profile"
                            attr:rel="noopener noreferrer"
                            attr:target="_blank"
                        >
                            <i class="fab fa-github text-xl" aria-hidden="true"></i>
                        </Button>
                    </div>
                </nav>
            </div>
        </header>
    }
}

#[derive(Clone, Copy)]
enum MenuBar {
    Top,
    Middle,
    Bottom,
}

fn menu_bar_class(menu_open: bool, bar: MenuBar) -> &'static str {
    match (menu_open, bar) {
        (true, MenuBar::Top) => {
            "block h-0.5 w-5 translate-y-2 rotate-45 rounded-full bg-current transition-transform"
        }
        (true, MenuBar::Middle) => {
            "block h-0.5 w-5 rounded-full bg-current opacity-0 transition-opacity"
        }
        (true, MenuBar::Bottom) => {
            "block h-0.5 w-5 -translate-y-2 -rotate-45 rounded-full bg-current transition-transform"
        }
        (false, _) => "block h-0.5 w-5 rounded-full bg-current transition-all",
    }
}
