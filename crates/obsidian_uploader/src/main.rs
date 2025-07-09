use anyhow::Result;
use obsidian_uploader::{Config, run_main};

#[tokio::main]
async fn main() -> Result<()> {
    // 設定を初期化
    let config = Config::new()?;

    // メイン処理を実行
    run_main(config).await?;

    Ok(())
}
