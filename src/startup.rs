use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use sqlx::PgPool;
use tokio::signal;

use crate::{
    config::Config,
    database::Database,
    object_storage::{self, ObjectStorage},
    route::{
        COVER_MAX_BYTES, delete_book, delete_book_cover, delete_creator, get_book,
        get_book_cover_image, get_book_covers, get_books, get_content_ratings, get_creator,
        get_creators, get_labels, get_languages, healthz, store_book, store_book_cover,
        store_creator, update_book, update_book_main_cover, update_creator,
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

        let storage = ObjectStorage::new(
            object_storage::client(&cfg.object_storage),
            cfg.object_storage.covers_bucket.clone(),
        );
        storage
            .ensure_bucket()
            .await
            .expect("failed to ensure the covers bucket");

        let service = Service::new(database, storage);

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
            .route("/books", post(store_book).get(get_books))
            .route(
                "/books/{id}",
                get(get_book).put(update_book).delete(delete_book),
            )
            .route(
                "/books/{id}/covers",
                get(get_book_covers)
                    .merge(post(store_book_cover).layer(DefaultBodyLimit::max(COVER_MAX_BYTES))),
            )
            .route("/books/{id}/covers/{cover_id}", delete(delete_book_cover))
            .route(
                "/books/{id}/covers/{cover_id}/image",
                get(get_book_cover_image),
            )
            .route("/books/{id}/main-cover", put(update_book_main_cover))
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
