use sqlx::{Postgres, QueryBuilder};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{Creator, CreatorQuery, CreatorRole, Filter, Name},
    error::{DatabaseError, LogInternal},
};

#[derive(sqlx::FromRow)]
pub(super) struct CreatorQueryRow {
    pub(super) id: Uuid,
    pub(super) first_name: String,
    pub(super) last_name: String,
    pub(super) roles: Vec<String>,
    pub(super) created_at: time::OffsetDateTime,
}

impl TryFrom<CreatorQueryRow> for CreatorQuery {
    type Error = DatabaseError;

    fn try_from(row: CreatorQueryRow) -> Result<Self, Self::Error> {
        let first_name = Name::try_from(row.first_name).or_corrupted("first_name")?;
        let last_name = Name::try_from(row.last_name).or_corrupted("last_name")?;

        let mut roles = Vec::with_capacity(row.roles.len());
        for role in row.roles {
            let role: CreatorRole = role.parse().or_corrupted("role")?;
            roles.push(role);
        }

        Ok(CreatorQuery {
            id: row.id,
            first_name,
            last_name,
            roles,
            created_at: row.created_at.into(),
        })
    }
}

impl Database {
    #[instrument(name = "db.creator.store", skip_all, fields(creator.id = %item.id))]
    pub async fn store_creator(&self, item: &Creator) -> Result<(), DatabaseError> {
        sqlx::query_file!(
            "queries/store_creator.sql",
            item.id,
            item.first_name.as_ref(),
            item.last_name.as_ref(),
            item.created_at.into_inner()
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "db.creator.update", skip_all, fields(creator.id = %item.id))]
    pub async fn update_creator(&self, item: &Creator) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!(
            "queries/update_creator.sql",
            item.id,
            item.first_name.as_ref(),
            item.last_name.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Creator>());
        }

        Ok(())
    }

    #[instrument(name = "db.creator.get", skip_all, fields(creator.id = %id))]
    pub async fn get_creator(&self, id: Uuid) -> Result<CreatorQuery, DatabaseError> {
        self.get_creator_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_creator_inner(&self, id: Uuid) -> Result<CreatorQuery, DatabaseError> {
        let row = sqlx::query_file_as!(CreatorQueryRow, "queries/get_creator.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => CreatorQuery::try_from(row),
            None => Err(DatabaseError::not_found::<Creator>()),
        }
    }

    #[instrument(name = "db.creator.list", skip_all, fields(filter = ?filter))]
    pub async fn get_creators(
        &self,
        filter: &Filter,
    ) -> Result<(Vec<CreatorQuery>, Option<Uuid>), DatabaseError> {
        self.get_creators_inner(filter)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_creators_inner(
        &self,
        filter: &Filter,
    ) -> Result<(Vec<CreatorQuery>, Option<Uuid>), DatabaseError> {
        let limit = filter.limit.as_i64();
        let sort_order = filter.sort_order;
        let fetch_limit = super::fetch_limit(limit);
        let (cursor_comparison, direction) = super::cursor_op(sort_order);

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r"select c.id, c.first_name, c.last_name, c.created_at,
                     coalesce(array_agg(distinct bc.role order by bc.role)
                              filter (where bc.role is not null), array[]::text[]) as roles
              from creators c
              left join book_creators bc on bc.creator_id = c.id",
        );

        if let Some(id) = filter.after {
            builder
                .push(" where c.id ")
                .push(cursor_comparison)
                .push_bind(id);
        }

        builder
            .push(" group by c.id order by c.id ")
            .push(direction)
            .push(" limit ")
            .push_bind(fetch_limit);

        let mut rows = builder
            .build_query_as::<CreatorQueryRow>()
            .fetch_all(&self.pool)
            .await?;

        let next_cursor = super::take_page(&mut rows, limit, |r| r.id);

        let mut creators: Vec<CreatorQuery> = Vec::with_capacity(rows.len());
        for row in rows {
            let creator = row.try_into()?;
            creators.push(creator);
        }

        Ok((creators, next_cursor))
    }

    #[instrument(name = "db.creator.delete", skip_all, fields(creator.id = %id))]
    pub async fn delete_creator(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_creator.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Creator>());
        }

        Ok(())
    }
}
