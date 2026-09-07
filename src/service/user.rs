use uuid::Uuid;

use crate::{
    entity::{User, UserQuery},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn register_user(&self, user: User) -> Result<(), ServiceError> {
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
        let password = user.password.clone();
        let password_hash =
            tokio::task::spawn_blocking(move || hasher.hash_password(password.expose_secret()))
                .await
                .expect("password hashing task panicked")?;

        self.database.store_user(user, password_hash).await?;

        Ok(())
    }

    pub async fn get_user(&self, id: Uuid) -> Result<UserQuery, ServiceError> {
        self.database.get_user(id).await.map_err(Into::into)
    }
}
