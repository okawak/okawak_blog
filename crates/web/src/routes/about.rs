use leptos::prelude::*;
use stylance::import_style;
use thaw::*;

import_style!(about_style, "about.module.scss");

/// Aboutページコンポーネント
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
                    // ブログについて
                    <div class=about_style::content_card>
                        <h2>{"ブログについて"}</h2>
                        <p>
                            {"気になったことをメモしておくブログです。技術的な発見、日常の学び、統計・物理学に関する考察などを記録しています。"}
                        </p>
                        <p>
                            {"主に個人的な学習記録として使用していますが、同じような興味を持つ方に少しでも参考になれば幸いです。"}
                        </p>
                    </div>

                // 技術スタック
                // <div class=about_style::content_card>
                // <h2>{"技術スタック"}</h2>
                // <div class=about_style::tech_stack>
                // <div class=about_style::tech_category>
                // <h3>{"フロントエンド"}</h3>
                // <div class=about_style::tech_list>
                // <Space vertical=true>
                // <div class=about_style::tech_item>
                // <strong>{"Leptos 0.8"}</strong>
                // {" - Rust製WebAssemblyフレームワーク"}
                // </div>
                // <div class=about_style::tech_item>
                // <strong>{"thaw-ui"}</strong>
                // {" - モダンUIコンポーネントライブラリ"}
                // </div>
                // <div class=about_style::tech_item>
                // <strong>{"stylance"}</strong>
                // {" - CSS-in-Rustスタイリング"}
                // </div>
                // </Space>
                // </div>
                // </div>

                // <div class=about_style::tech_category>
                // <h3>{"バックエンド・インフラ"}</h3>
                // <div class=about_style::tech_list>
                // <Space vertical=true>
                // <div class=about_style::tech_item>
                // <strong>{"Rust"}</strong>
                // {" - システムプログラミング言語"}
                // </div>
                // <div class=about_style::tech_item>
                // <strong>{"AWS S3"}</strong>
                // {" - 静的ファイルホスティング"}
                // </div>
                // <div class=about_style::tech_item>
                // <strong>{"Terraform"}</strong>
                // {" - インフラ構成管理"}
                // </div>
                // </Space>
                // </div>
                // </div>
                // </div>
                // </div>

                // // 開発者について
                // <div class=about_style::content_card>
                // <h2>{"開発者について"}</h2>
                // <p>
                // {"ソフトウェア開発者として、特にRustエコシステムとWebアセンブリ技術に興味を持っています。"}
                // </p>
                // <p>
                // {"統計学、物理学、数学などの理系分野にも関心があり、技術と学術の両面から様々なトピックを探求しています。"}
                // </p>
                // <div class=about_style::contact_actions>
                // <Space>
                // <Button>{"GitHub"}</Button>
                // <Button>{"Contact"}</Button>
                // </Space>
                // </div>
                // </div>

                // // このサイトの特徴
                // <div class=about_style::content_card>
                // <h2>{"このサイトの特徴"}</h2>
                // <div class=about_style::feature_list>
                // <Space vertical=true>
                // <div class=about_style::feature_item>
                // <h4>{"Rust + WebAssembly"}</h4>
                // <p>{"高性能なフロントエンドを実現"}</p>
                // </div>
                // <div class=about_style::feature_item>
                // <h4>{"ハイブリッドスタイリング"}</h4>
                // <p>{"stylanceとthaw-uiの統合デザインシステム"}</p>
                // </div>
                // <div class=about_style::feature_item>
                // <h4>{"レスポンシブデザイン"}</h4>
                // <p>{"モバイルファーストのアクセシブルなUI"}</p>
                // </div>
                // <div class=about_style::feature_item>
                // <h4>{"モダン開発体験"}</h4>
                // <p>{"型安全性とパフォーマンスを両立"}</p>
                // </div>
                // </Space>
                // </div>
                // </div>
                </div>
            </section>

        // デモセクション（技術展示）
        // <section class=about_style::demo_section>
        // <h2>{"技術デモ"}</h2>
        // <div class=about_style::demo_grid>
        // <div class=about_style::demo_card>
        // <h3>{"インタラクティブコンポーネント"}</h3>
        // <p>{"thaw-uiコンポーネントのデモンストレーション"}</p>
        // <Space>
        // <Button>{"プライマリ"}</Button>
        // <Button>{"セカンダリ"}</Button>
        // </Space>
        // </div>
        // <div class=about_style::demo_card>
        // <h3>{"アニメーション効果"}</h3>
        // <p>{"CSS Custom Propertiesによるスムーズなアニメーション"}</p>
        // <div class=about_style::animation_demo>
        // {"ホバーしてみてください"}
        // </div>
        // </div>
        // <div class=about_style::demo_card>
        // <h3>{"レスポンシブレイアウト"}</h3>
        // <p>{"画面サイズに応じたフレキシブルなグリッドシステム"}</p>
        // <div class=about_style::responsive_indicator>
        // {"画面サイズ: 自動調整中"}
        // </div>
        // </div>
        // </div>
        // </section>
        </div>
    }
}
