use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Email, Password, Role, User, Username},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn register_user(
        &self,
        id: Uuid,
        email: Email,
        username: Username,
        password: Password,
        created_at: OffsetDateTime,
    ) -> Result<User, ServiceError> {
        // Reject a known address before paying for Argon2; `unique_users_email`
        // is still the backstop for a race past this point.
        if self.database.get_user_by_email(&email).await?.is_some() {
            return Err(ServiceError::AlreadyExists("User email"));
        }

        // Argon2 blocks the runtime; hash off the reactor.
        let hasher = self.hasher.clone();
        let password_hash =
            tokio::task::spawn_blocking(move || hasher.hash_password(password.expose_secret()))
                .await
                .expect("password hashing task panicked")?;

        let user = User {
            id,
            email,
            username,
            password_hash,
            roles: vec![Role::Reader],
            verified_at: None,
            created_at,
        };

        self.database.store_user(&user).await?;

        Ok(user)
    }

    pub async fn get_user(&self, id: Uuid) -> Result<User, ServiceError> {
        self.database.get_user(id).await.map_err(Into::into)
    }
}
