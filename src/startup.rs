use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tokio::signal;

use crate::{
    config::Config,
    database::Database,
    route::{
        delete_creator, get_content_ratings, get_creator, get_creators, get_labels, get_languages,
        healthz, store_creator, update_creator,
    },
    service::Service,
};

pub struct App {
    router: Router,
    addr: String,
}

impl App {
    pub async fn build(cfg: &Config) -> Self {
        let pool = PgPool::connect_with(cfg.database.with_db())
            .await
            .expect("failed to connect to Postgres");
        let database = Database::new(pool);
        let service = Service::new(database);

        let router = Router::new()
            .route("/healthz", get(healthz))
            .route("/content-ratings", get(get_content_ratings))
            .route("/languages", get(get_languages))
            .route("/labels", get(get_labels))
            .route("/creators", post(store_creator).get(get_creators))
            .route(
                "/creators/{id}",
                get(get_creator).put(update_creator).delete(delete_creator),
            )
            .with_state(service);

        Self {
            router,
            addr: cfg.addr.clone(),
        }
    }

    pub fn router(&self) -> Router {
        self.router.clone()
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let listener = tokio::net::TcpListener::bind(self.addr).await?;

        axum::serve(listener, self.router)
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
