use manga_theka::{config::Config, startup::App, telemetry};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let cfg = Config::from_env();

    telemetry::init_subsciber(cfg.log_level.clone());
    tracing::info!("listening on {}", cfg.addr);

    let app = App::build(cfg).await;
    app.serve().await?;

    Ok(())
}
