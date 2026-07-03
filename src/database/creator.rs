use crate::{entity::Creator, error::DatabaseError};

#[derive(Debug, Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn store_creator(&self, item: Creator) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
insert into	creators(id, first_name, last_name, role, created_at)
values($1, $2, $3, $4, $5);
            "#,
            item.id,
            item.first_name.as_ref(),
            item.last_name.as_ref(),
            item.role as _,
            item.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(|e| {
            if DatabaseError::is_unknown(e) {
                tracing::error!("failed to store new creator in database: {e:?}");
            }
        })?;

        Ok(())
    }
}
