use axum::{Router, routing::get};
use tokio::{net::TcpListener, signal};

use crate::routes::healthz;

pub struct App {
    router: Router,
    listener: TcpListener,
}

impl App {
    pub async fn build(addr: &str) -> Result<Self, std::io::Error> {
        let router = Router::new().route("/healthz", get(healthz));
        let listener = tokio::net::TcpListener::bind(addr).await?;

        Ok(Self { router, listener })
    }

    pub fn router(&self) -> Router {
        self.router.clone()
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(shutdown_signal())
            .await
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
