use manga_theka::{startup::App, telemetry};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    telemetry::init_subsciber("info".into());

    let addr = String::from("0.0.0.0:3000");
    let app = App::build(&addr).await?;

    tracing::info!("listening on {}", addr);
    app.run().await?;

    Ok(())
}
