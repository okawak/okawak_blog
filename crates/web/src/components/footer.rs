use leptos::prelude::*;
use stylance::import_style;

import_style!(footer_style, "footer.module.scss");

/// サイトフッターコンポーネント
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class=footer_style::site_footer>
            <div class=footer_style::copyright>
                <p>{format!("© {} okawak. All Rights Reserved.", current_year())}</p>
                <p>
                    <small>
                        Powered by
                        <a href="https://leptos.dev" target="_blank" rel="noopener noreferrer">
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
