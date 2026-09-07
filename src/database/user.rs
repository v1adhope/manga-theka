use time::OffsetDateTime;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{Email, PasswordHash, Role, User, Username},
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
pub(super) struct UserRow {
    pub(super) id: Uuid,
    pub(super) email: String,
    pub(super) username: String,
    pub(super) password_hash: String,
    pub(super) roles: Vec<String>,
    pub(super) verified_at: Option<OffsetDateTime>,
    pub(super) created_at: OffsetDateTime,
}

impl TryFrom<UserRow> for User {
    type Error = DatabaseError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let email = Email::try_from(row.email).or_corrupted("email")?;
        let username = Username::try_from(row.username).or_corrupted("username")?;
        let password_hash =
            PasswordHash::try_from(row.password_hash).or_corrupted("password_hash")?;

        let mut roles = Vec::with_capacity(row.roles.len());
        for role in row.roles {
            let role: Role = role.parse().or_corrupted("roles")?;
            roles.push(role);
        }

        Ok(User {
            id: row.id,
            email,
            username,
            password_hash,
            roles,
            verified_at: row.verified_at,
            created_at: row.created_at,
        })
    }
}

impl Database {
    #[instrument(name = "db.user.store", skip_all, fields(user.id = %item.id))]
    pub async fn store_user(&self, item: &User) -> Result<(), DatabaseError> {
        let roles: Vec<&str> = item.roles.iter().map(AsRef::as_ref).collect();

        sqlx::query_file!(
            "queries/store_user.sql",
            item.id,
            item.email.as_ref(),
            item.username.as_ref(),
            item.password_hash.as_ref(),
            &roles as &[&str],
            item.verified_at,
            item.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "db.user.get", skip_all, fields(user.id = %id))]
    pub async fn get_user(&self, id: Uuid) -> Result<User, DatabaseError> {
        self.get_user_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_user_inner(&self, id: Uuid) -> Result<User, DatabaseError> {
        let row = sqlx::query_file_as!(UserRow, "queries/get_user.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => User::try_from(row),
            None => Err(DatabaseError::not_found::<User>()),
        }
    }

    #[instrument(name = "db.user.get_by_email", skip_all)]
    pub async fn get_user_by_email(&self, email: &Email) -> Result<Option<User>, DatabaseError> {
        self.get_user_by_email_inner(email)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_user_by_email_inner(&self, email: &Email) -> Result<Option<User>, DatabaseError> {
        let row = sqlx::query_file_as!(UserRow, "queries/get_user_by_email.sql", email.as_ref())
            .fetch_optional(&self.pool)
            .await?;

        row.map(User::try_from).transpose()
    }
}
