use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    entity::{Creator, CreatorRole, Name, Pagination},
    error::DatabaseError,
};

#[derive(Debug, Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
}

#[derive(sqlx::FromRow)]
struct CreatorRow {
    id: Uuid,
    first_name: String,
    last_name: String,
    role: String,
    created_at: time::OffsetDateTime,
}

impl TryFrom<CreatorRow> for Creator {
    type Error = DatabaseError;

    fn try_from(row: CreatorRow) -> Result<Self, Self::Error> {
        let role: CreatorRole = row
            .role
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("role", e))?;
        let first_name = Name::try_from(row.first_name)
            .map_err(|e| DatabaseError::invariant_corrupted("first_name", e))?;
        let last_name = Name::try_from(row.last_name)
            .map_err(|e| DatabaseError::invariant_corrupted("last_name", e))?;

        Ok(Creator {
            id: row.id,
            first_name,
            last_name,
            role,
            created_at: row.created_at,
        })
    }
}

// TODO: tune tracing
impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn store_creator(&self, item: Creator) -> Result<(), DatabaseError> {
        sqlx::query_file!(
            "queries/store_creator.sql",
            item.id,
            item.first_name.as_ref(),
            item.last_name.as_ref(),
            item.role.as_ref() as _,
            item.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to store new creator in database: {e:?}");
            }
        })?;

        Ok(())
    }

    pub async fn update_creator(&self, item: Creator) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!(
            "queries/update_creator.sql",
            item.id,
            item.first_name.as_ref(),
            item.last_name.as_ref(),
            item.role.as_ref() as _,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to update creator in database: {e:?}");
            }
        })?;

        if row.is_none() {
            return Err(DatabaseError::CreatorNotFound);
        }
        Ok(())
    }
    pub async fn get_creator(&self, id: Uuid) -> Result<Creator, DatabaseError> {
        let row = sqlx::query_file_as!(CreatorRow, "queries/get_creator.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get creator from database: {e:?}");
                }
            })?;

        match row {
            Some(row) => row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert CreatorRow: {e:?}");
                }
            }),
            None => Err(DatabaseError::CreatorNotFound),
        }
    }

    pub async fn get_creators(
        &self,
        pagination: &Pagination,
    ) -> Result<(Vec<Creator>, Option<Uuid>), DatabaseError> {
        let limit = pagination.limit.as_u32();
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("select id, first_name, last_name, role, created_at from creators");

        if let Some(id) = pagination.after {
            builder.push(" where id < ").push_bind(id);
        }

        builder
            .push(" order by id desc limit ")
            .push_bind((limit + 1) as i64);

        let rows = builder
            .build_query_as::<CreatorRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get creators from database: {e:?}");
                }
            })?;

        let mut creators: Vec<Creator> = Vec::with_capacity(rows.len());
        for row in rows {
            let creator = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert creator row: {e:?}");
                }
            })?;
            creators.push(creator);
        }

        let mut next_cursor = None;
        if creators.len() > limit as usize {
            creators.pop();
            next_cursor = creators.last().map(|c| c.id);
        }

        Ok((creators, next_cursor))
    }

    pub async fn delete_creator(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_creator.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to delete creator from database: {e:?}");
                }
            })?;

        if row.is_none() {
            return Err(DatabaseError::CreatorNotFound);
        }
        Ok(())
    }
}
