use manga_theka::{startup::App, telemetry};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    telemetry::init_subsciber("info".into());

    // TODO: config
    let addr = "0.0.0.0:3000";
    let pg_url = "postgres://postgres:postgres@localhost:5432/manga_theka";

    let app = App::build(addr, pg_url).await?;

    tracing::info!("listening on {}", addr);
    app.run().await?;

    Ok(())
}
