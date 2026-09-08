use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
};
use secrecy::ExposeSecret;
use tokio::signal;

use crate::{
    config::Config,
    database::{self, Database},
    entity::{DEFAULT_IMAGE_MAX_BYTES, Role, UPLOAD_MAX_BYTES},
    hasher::Hasher,
    jwt::Jwt,
    memory_storage::{self, MemoryStore},
    middleware::{authenticate, require_roles},
    object_storage::{self, ObjectStorage},
    route::{
        commit_chapter_release, delete_book, delete_book_cover, delete_chapter,
        delete_chapter_release, delete_creator, get_book, get_book_cover_image, get_book_covers,
        get_books, get_chapter, get_chapter_page, get_chapter_page_image, get_chapter_pages,
        get_chapter_release, get_chapter_releases, get_chapters, get_content_ratings, get_creator,
        get_creators, get_feedback, get_feedbacks, get_labels, get_languages, get_me, healthz,
        list_my_sessions, login, refresh, register, revoke_all_sessions, revoke_current_session,
        revoke_session, store_book, store_book_cover, store_chapter, store_chapter_release,
        store_creator, store_feedback, update_book, update_book_main_cover, update_book_visibility,
        update_chapter, update_creator, update_feedback_status, upload_chapter_pages,
    },
    service::Service,
};

const CONTENT_WRITERS: &[Role] = &[Role::Uploader, Role::Moderator, Role::Admin];
const MODERATORS: &[Role] = &[Role::Moderator, Role::Admin];
const SIGNED_IN: &[Role] = &[Role::Reader, Role::Uploader, Role::Moderator, Role::Admin];

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

        let hasher = Hasher::new(
            cfg.auth.password.m_cost,
            cfg.auth.password.t_cost,
            cfg.auth.password.p_cost,
            cfg.auth.pepper.key.expose_secret().as_bytes(),
        )
        .expect("failed to build the password hasher");

        let jwt = Jwt::load(&cfg.auth.jwt_access, &cfg.auth.jwt_refresh);

        let redis = memory_storage::connection(&cfg.redis).await;
        let memory = MemoryStore::new(redis, cfg.auth.jwt_refresh.ttl);

        let service = Service::new(database, storage, hasher, jwt, memory);

        let router = Router::new()
            .route("/healthz", get(healthz))
            .route("/content-ratings", get(get_content_ratings))
            .route("/languages", get(get_languages))
            .route("/labels", get(get_labels))
            .route("/users/register", post(register))
            .route("/users/me", get(get_me))
            .route("/sessions/login", post(login))
            .route("/sessions/refresh", post(refresh))
            .route("/sessions/me", get(list_my_sessions))
            .route("/sessions/me/all", delete(revoke_all_sessions))
            .route("/sessions/me/current", delete(revoke_current_session))
            .route("/sessions/me/{sid}", delete(revoke_session))
            .route(
                "/creators",
                post(store_creator)
                    .layer(require_roles(CONTENT_WRITERS))
                    .merge(get(get_creators)),
            )
            .route(
                "/creators/{id}",
                get(get_creator)
                    .merge(put(update_creator).layer(require_roles(CONTENT_WRITERS)))
                    .merge(delete(delete_creator).layer(require_roles(MODERATORS))),
            )
            .route(
                "/books",
                post(store_book)
                    .layer(require_roles(SIGNED_IN))
                    .merge(get(get_books)),
            )
            .route(
                "/books/{id}",
                get(get_book)
                    .merge(put(update_book).layer(require_roles(CONTENT_WRITERS)))
                    .merge(delete(delete_book).layer(require_roles(MODERATORS))),
            )
            .route(
                "/books/{id}/covers",
                get(get_book_covers).merge(
                    post(store_book_cover)
                        .layer(DefaultBodyLimit::max(DEFAULT_IMAGE_MAX_BYTES))
                        .layer(require_roles(CONTENT_WRITERS)),
                ),
            )
            .route(
                "/covers/{id}",
                delete(delete_book_cover).layer(require_roles(CONTENT_WRITERS)),
            )
            .route("/covers/{id}/image", get(get_book_cover_image))
            .route(
                "/books/{id}/main-cover",
                put(update_book_main_cover).layer(require_roles(CONTENT_WRITERS)),
            )
            .route("/books/{id}/visibility", put(update_book_visibility))
            .route(
                "/books/{id}/chapters",
                post(store_chapter)
                    .layer(require_roles(CONTENT_WRITERS))
                    .merge(get(get_chapters)),
            )
            .route(
                "/chapters/{id}",
                get(get_chapter).merge(
                    put(update_chapter)
                        .delete(delete_chapter)
                        .layer(require_roles(CONTENT_WRITERS)),
                ),
            )
            .route(
                "/chapters/{id}/releases",
                post(store_chapter_release)
                    .layer(require_roles(CONTENT_WRITERS))
                    .merge(get(get_chapter_releases)),
            )
            .route(
                "/releases/{id}",
                get(get_chapter_release)
                    .merge(delete(delete_chapter_release).layer(require_roles(CONTENT_WRITERS))),
            )
            .route(
                "/releases/{id}/upload",
                post(upload_chapter_pages)
                    .layer(DefaultBodyLimit::max(UPLOAD_MAX_BYTES))
                    .layer(require_roles(CONTENT_WRITERS)),
            )
            .route(
                "/releases/{id}/commit",
                post(commit_chapter_release).layer(require_roles(CONTENT_WRITERS)),
            )
            .route("/releases/{id}/pages", get(get_chapter_pages))
            .route("/releases/{id}/pages/{page_number}", get(get_chapter_page))
            .route(
                "/releases/{id}/pages/{page_id}/image",
                get(get_chapter_page_image),
            )
            .route(
                "/feedbacks",
                post(store_feedback).merge(get(get_feedbacks).layer(require_roles(MODERATORS))),
            )
            .route(
                "/feedbacks/{id}",
                get(get_feedback).layer(require_roles(MODERATORS)),
            )
            .route(
                "/feedbacks/{id}/status",
                put(update_feedback_status).layer(require_roles(MODERATORS)),
            )
            .layer(from_fn_with_state(service.clone(), authenticate))
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
