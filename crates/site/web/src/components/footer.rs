use leptos::prelude::*;

/// Site footer component.
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="border-t border-border bg-gradient-to-r from-card to-background px-4 py-8 text-center text-sm text-muted-foreground">
            <div class="mx-auto max-w-[var(--site-content-width)]">
                <p class="my-2 leading-relaxed">
                    {format!("© {} okawak. All Rights Reserved.", current_year())}
                </p>
                <p class="my-2 leading-relaxed">
                    <small>
                        "Powered by "
                        <a
                            class="text-primary no-underline transition-colors hover:text-[var(--color-primary-hover)] focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                            href="https://leptos.dev"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            Leptos
                        </a>
                    </small>
                </p>
            </div>
        </footer>
    }
}

fn current_year() -> String {
    use chrono::Datelike;
    let now = chrono::Local::now();
    now.year().to_string()
}
