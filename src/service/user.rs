use uuid::Uuid;

use crate::{
    entity::{User, UserQuery},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn register_user(&self, user: User) -> Result<(), ServiceError> {
        self.database.ensure_email_available(&user.email).await?;

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
