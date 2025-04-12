use notion_api::{load_config, run_main};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 環境変数から設定情報を読み込む
    let config = load_config()?;
    // lib.rsのrun_main関数を呼び出す
    run_main(config).await
}
