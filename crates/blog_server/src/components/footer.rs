use leptos::prelude::*;

/// サイトフッターコンポーネント
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <div class="footer-container">
                <div class="footer-content">
                    <div class="footer-section">
                        <h3>ぶくせんの探窟メモ</h3>
                        <p>
                            物理学, 統計学, 日常, 技術的な事柄についての個人的なメモです.
                        </p>
                    </div>

                    <div class="footer-section">
                        <h3>カテゴリー</h3>
                        <ul class="footer-links">
                            <li>
                                <a href="/statistics">統計学</a>
                            </li>
                            <li>
                                <a href="/physics">物理学</a>
                            </li>
                            <li>
                                <a href="/daily">日常</a>
                            </li>
                            <li>
                                <a href="/tech">技術</a>
                            </li>
                        </ul>
                    </div>

                    <div class="footer-section">
                        <h3>リンク</h3>
                        <ul class="footer-links">
                            <li>
                                <a
                                    href="https://github.com/okawak"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >
                                    GitHub
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://twitter.com/okawak_"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >
                                    Twitter
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>

                <div class="copyright">
                    <p>&copy; {current_year()}okawak. All Rights Reserved.</p>
                    <p>
                        <small>
                            Powered by
                            <a href="https://leptos.dev" target="_blank" rel="noopener noreferrer">
                                Leptos
                            </a>
                        </small>
                    </p>
                </div>
            </div>
        </footer>
    }
}

/// 現在の年を返す関数
fn current_year() -> String {
    // システムの現在時刻から年を取得
    use chrono::Datelike;
    let now = chrono::Local::now();
    now.year().to_string()
}
