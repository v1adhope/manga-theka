use time::OffsetDateTime;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{Email, PasswordHash, Roles, User, UserCredentials, UserQuery, Username},
    error::{DatabaseError, LogInternal},
};

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    username: String,
    roles: Vec<String>,
    verified_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl TryFrom<UserRow> for UserQuery {
    type Error = DatabaseError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let email = Email::try_from(row.email).or_corrupted("email")?;
        let username = Username::try_from(row.username).or_corrupted("username")?;

        Ok(UserQuery {
            id: row.id,
            email,
            username,
            roles: Roles::try_from(row.roles).or_corrupted("roles")?,
            verified_at: row.verified_at.map(Into::into),
            created_at: row.created_at.into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct CredentialsRow {
    id: Uuid,
    password_hash: String,
    roles: Vec<String>,
}

impl TryFrom<CredentialsRow> for UserCredentials {
    type Error = DatabaseError;

    fn try_from(row: CredentialsRow) -> Result<Self, Self::Error> {
        let password_hash =
            PasswordHash::try_from(row.password_hash).or_corrupted("password_hash")?;

        Ok(UserCredentials {
            id: row.id,
            password_hash,
            roles: Roles::try_from(row.roles).or_corrupted("roles")?,
        })
    }
}

impl Database {
    #[instrument(name = "db.user.store", skip_all, fields(user.id = %user.id))]
    pub async fn store_user(
        &self,
        user: User,
        password_hash: PasswordHash,
    ) -> Result<(), DatabaseError> {
        sqlx::query_file!(
            "queries/store_user.sql",
            user.id,
            user.email.as_ref(),
            user.username.as_ref(),
            password_hash.as_ref(),
            user.created_at.into_inner(),
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "db.user.get", skip_all, fields(user.id = %id))]
    pub async fn get_user(&self, id: Uuid) -> Result<UserQuery, DatabaseError> {
        self.get_user_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_user_inner(&self, id: Uuid) -> Result<UserQuery, DatabaseError> {
        let row = sqlx::query_file_as!(UserRow, "queries/get_user.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => UserQuery::try_from(row),
            None => Err(DatabaseError::not_found::<User>()),
        }
    }

    #[instrument(name = "db.user.get_roles", skip_all, fields(user.id = %id))]
    pub async fn get_user_roles(&self, id: Uuid) -> Result<Roles, DatabaseError> {
        self.get_user_roles_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_user_roles_inner(&self, id: Uuid) -> Result<Roles, DatabaseError> {
        let roles = sqlx::query_file_scalar!("queries/get_user_roles.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        match roles {
            Some(roles) => Roles::try_from(roles).or_corrupted("roles"),
            None => Err(DatabaseError::not_found::<User>()),
        }
    }

    #[instrument(name = "db.user.get_credentials_by_email", skip_all)]
    pub async fn get_user_credentials_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<UserCredentials>, DatabaseError> {
        self.get_user_credentials_by_email_inner(email)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_user_credentials_by_email_inner(
        &self,
        email: &Email,
    ) -> Result<Option<UserCredentials>, DatabaseError> {
        let row = sqlx::query_file_as!(
            CredentialsRow,
            "queries/get_user_credentials_by_email.sql",
            email.as_ref()
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(UserCredentials::try_from).transpose()
    }

    #[instrument(name = "db.user.ensure_identity_available", skip_all)]
    pub async fn ensure_identity_available(
        &self,
        email: &Email,
        username: &Username,
    ) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!(
            "queries/user_identity_available.sql",
            email.as_ref(),
            username.as_ref(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        if row.email_taken {
            return Err(DatabaseError::taken("email"));
        }

        if row.username_taken {
            return Err(DatabaseError::taken("username"));
        }

        Ok(())
    }
}
