use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use tokio::signal;

use crate::{
    config::Config,
    database::{self, Database},
    entity::{DEFAULT_IMAGE_MAX_BYTES, UPLOAD_MAX_BYTES},
    object_storage::{self, ObjectStorage},
    route::{
        commit_chapter_release, delete_book, delete_book_cover, delete_chapter,
        delete_chapter_release, delete_creator, get_book, get_book_cover_image, get_book_covers,
        get_books, get_chapter, get_chapter_page_image, get_chapter_pages, get_chapter_release,
        get_chapter_releases, get_chapters, get_content_ratings, get_creator, get_creators,
        get_labels, get_languages, get_staged_chapter_pages, healthz, store_book, store_book_cover,
        store_chapter, store_chapter_release, store_creator, update_book, update_book_main_cover,
        update_chapter, update_creator, upload_chapter_pages,
    },
    service::Service,
};

pub struct App {
    router: Router,
    addr: String,
}

impl App {
    pub async fn build(cfg: &Config) -> Self {
        let pool = database::pool(&cfg.database).await;
        let database = Database::new(pool);
        database.migrate().await;

        let client = object_storage::client(&cfg.object_storage).await;
        let storage = ObjectStorage::new(
            client,
            cfg.object_storage.covers_bucket.clone(),
            cfg.object_storage.release_pages_bucket.clone(),
        );
        storage.ensure_buckets().await;

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
                get(get_book_covers).merge(
                    post(store_book_cover).layer(DefaultBodyLimit::max(DEFAULT_IMAGE_MAX_BYTES)),
                ),
            )
            .route("/books/{id}/covers/{cover_id}", delete(delete_book_cover))
            .route(
                "/books/{id}/covers/{cover_id}/image",
                get(get_book_cover_image),
            )
            .route("/books/{id}/main-cover", put(update_book_main_cover))
            .route(
                "/books/{id}/chapters",
                post(store_chapter).get(get_chapters),
            )
            .route(
                "/chapters/{id}",
                get(get_chapter).put(update_chapter).delete(delete_chapter),
            )
            .route(
                "/chapters/{id}/releases",
                post(store_chapter_release).get(get_chapter_releases),
            )
            .route(
                "/releases/{id}",
                get(get_chapter_release).delete(delete_chapter_release),
            )
            .route(
                "/releases/{id}/upload",
                post(upload_chapter_pages).layer(DefaultBodyLimit::max(UPLOAD_MAX_BYTES)),
            )
            .route("/releases/{id}/commit", post(commit_chapter_release))
            .route("/releases/{id}/pages", get(get_chapter_pages))
            .route("/releases/{id}/staged", get(get_staged_chapter_pages))
            .route(
                "/releases/{id}/pages/{page_id}/image",
                get(get_chapter_page_image),
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
