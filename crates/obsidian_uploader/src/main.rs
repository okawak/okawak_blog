use anyhow::Result;
use obsidian_uploader::{Config, run_main};

#[tokio::main]
async fn main() -> Result<()> {
    // 設定を読み込み
    let config = Config::from_env()?;
    
    // メイン処理を実行
    run_main(config).await?;
    
    Ok(())
}
