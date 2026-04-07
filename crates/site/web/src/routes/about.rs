use leptos::prelude::*;
use stylance::import_style;

import_style!(about_style, "about.module.scss");

/// About page component.
#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <div class=about_style::about_page>
            <section class=about_style::hero_section>
                <div class=about_style::hero_content>
                    <h1>{"About"}</h1>
                    <p class=about_style::hero_description>
                        {"このブログについて、技術スタック、開発者について紹介します。"}
                    </p>
                </div>
            </section>

            <section class=about_style::content_section>
                <div class=about_style::card_grid>
                    // About the blog
                    <div class=about_style::content_card>
                        <h2>{"ブログについて"}</h2>
                        <p>
                            {"気になったことをメモしておくブログです。技術的な発見、日常の学び、統計・物理学に関する考察などを記録しています。"}
                        </p>
                        <p>
                            {"主に個人的な学習記録として使用していますが、同じような興味を持つ方に少しでも参考になれば幸いです。"}
                        </p>
                    </div>

                </div>
            </section>
        </div>
    }
}
