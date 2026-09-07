use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Role, User, UserQuery},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn register_user(
        &self,
        id: Uuid,
        user: User,
        created_at: OffsetDateTime,
    ) -> Result<UserQuery, ServiceError> {
        // Reject a known address before paying for Argon2; `unique_users_email`
        // is still the backstop for a race past this point.
        if self
            .database
            .get_user_by_email(&user.email)
            .await?
            .is_some()
        {
            return Err(ServiceError::AlreadyExists("User email"));
        }

        // Argon2 blocks the runtime; hash off the reactor.
        let hasher = self.hasher.clone();
        let password = user.password;
        let password_hash =
            tokio::task::spawn_blocking(move || hasher.hash_password(password.expose_secret()))
                .await
                .expect("password hashing task panicked")?;

        let stored = UserQuery {
            id,
            email: user.email,
            username: user.username,
            password_hash,
            roles: vec![Role::Reader],
            verified_at: None,
            created_at: created_at.into(),
        };

        self.database.store_user(&stored).await?;

        Ok(stored)
    }

    pub async fn get_user(&self, id: Uuid) -> Result<UserQuery, ServiceError> {
        self.database.get_user(id).await.map_err(Into::into)
    }
}
