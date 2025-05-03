use notion_api::{Result, load_config, run_main};

#[tokio::main]
async fn main() -> Result<()> {
    // 環境変数から設定情報を読み込む
    let config = load_config()?;
    // lib.rsのrun_main関数を呼び出す
    run_main(config).await
}
