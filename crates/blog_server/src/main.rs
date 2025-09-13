//! Blog Server Main - サーバー起動エントリーポイント

use blog_server::{config::Config, create_app, server::initialize_services};
use std::net::SocketAddr;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ログ初期化
    tracing_subscriber::fmt::init();

    // 設定読み込み
    let config = Config::load()?;
    tracing::info!("Configuration loaded: {:?}", config);

    // AWS SDK 初期化
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(config.aws.region.clone()))
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    // サービス初期化
    let (repository, storage, search) =
        initialize_services(s3_client, config.aws.s3_bucket.clone()).await;

    // Axum アプリケーション作成
    let app = create_app(repository, storage, search);

    // サーバー起動
    let addr = SocketAddr::new(config.server.host.parse()?, config.server.port);

    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
