use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{Creator, CreatorRole, DEFAULT_LIMIT, Limit, Name, Pagination},
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
pub(super) struct CreatorRow {
    pub(super) id: Uuid,
    pub(super) first_name: String,
    pub(super) last_name: String,
    pub(super) role: String,
    pub(super) created_at: time::OffsetDateTime,
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

impl Database {
    pub async fn store_creator(&self, item: &Creator) -> Result<(), DatabaseError> {
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

    pub async fn update_creator(&self, item: &Creator) -> Result<(), DatabaseError> {
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
                    tracing::error!("failed to convert creator row: {e:?}");
                }
            }),
            None => Err(DatabaseError::CreatorNotFound),
        }
    }

    pub async fn get_creators(
        &self,
        pagination: &Pagination,
    ) -> Result<(Vec<Creator>, Option<Uuid>), DatabaseError> {
        let limit = pagination.limit.map_or(DEFAULT_LIMIT, Limit::as_u32);
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("select id, first_name, last_name, role, created_at from creators");

        if let Some(id) = pagination.after {
            builder.push(" where id < ").push_bind(id);
        }

        builder
            .push(" order by id desc limit ")
            .push_bind((limit + 1) as i64);

        let mut rows = builder
            .build_query_as::<CreatorRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get creators from database: {e:?}");
                }
            })?;

        let mut next_cursor = None;
        if rows.len() > limit as usize {
            rows.pop();
            next_cursor = rows.last().map(|r| r.id);
        }

        let mut creators: Vec<Creator> = Vec::with_capacity(rows.len());
        for row in rows {
            let creator = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert creator row: {e:?}");
                }
            })?;
            creators.push(creator);
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
