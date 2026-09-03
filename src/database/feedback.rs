use sqlx::{Postgres, QueryBuilder};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{
        Email, Feedback, FeedbackFilter, FeedbackKind, FeedbackStatus, FeedbackStatusUpdate, Text,
    },
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
struct FeedbackRow {
    id: Uuid,
    kind: String,
    status: String,
    email: String,
    note: String,
    book_id: Option<Uuid>,
    updated_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

impl TryFrom<FeedbackRow> for Feedback {
    type Error = DatabaseError;

    fn try_from(row: FeedbackRow) -> Result<Self, Self::Error> {
        let kind: FeedbackKind = row.kind.parse().or_corrupted("kind")?;
        let status: FeedbackStatus = row.status.parse().or_corrupted("status")?;
        let email = Email::try_from(row.email).or_corrupted("email")?;
        let note = Text::try_from(row.note).or_corrupted("note")?;

        Ok(Feedback {
            id: row.id,
            kind,
            status,
            email,
            note,
            book_id: row.book_id,
            updated_at: row.updated_at,
            created_at: row.created_at,
        })
    }
}

impl Database {
    #[instrument(name = "db.feedback.store", skip_all, fields(feedback.id = %item.id))]
    pub async fn store_feedback(&self, item: &Feedback) -> Result<(), DatabaseError> {
        sqlx::query_file!(
            "queries/store_feedback.sql",
            item.id,
            item.kind.as_ref(),
            item.status.as_ref(),
            item.email.as_ref(),
            item.note.as_ref(),
            item.book_id,
            item.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "db.feedback.get", skip_all, fields(feedback.id = %id))]
    pub async fn get_feedback(&self, id: Uuid) -> Result<Feedback, DatabaseError> {
        self.get_feedback_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_feedback_inner(&self, id: Uuid) -> Result<Feedback, DatabaseError> {
        let row = sqlx::query_file_as!(FeedbackRow, "queries/get_feedback.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Feedback::try_from(row),
            None => Err(DatabaseError::not_found::<Feedback>()),
        }
    }

    #[instrument(name = "db.feedback.list", skip_all, fields(filter = ?filter))]
    pub async fn get_feedbacks(
        &self,
        filter: &FeedbackFilter,
    ) -> Result<(Vec<Feedback>, Option<Uuid>), DatabaseError> {
        self.get_feedbacks_inner(filter)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_feedbacks_inner(
        &self,
        filter: &FeedbackFilter,
    ) -> Result<(Vec<Feedback>, Option<Uuid>), DatabaseError> {
        let limit = filter.page.limit.as_i64();
        let sort_order = filter.page.sort_order;
        let fetch_limit = super::fetch_limit(limit);
        let (cursor_comparison, direction) = super::cursor_op(sort_order);

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r"select f.id, f.kind, f.status, f.email, f.note, f.book_id, f.updated_at, f.created_at
              from feedback f
              where true",
        );

        if let Some(kind) = &filter.kind {
            builder.push(" and f.kind = ").push_bind(kind.as_ref());
        }

        if let Some(status) = &filter.status {
            builder.push(" and f.status = ").push_bind(status.as_ref());
        }

        if let Some(cursor) = filter.page.after {
            builder
                .push(" and f.id ")
                .push(cursor_comparison)
                .push_bind(cursor);
        }

        builder
            .push(" order by f.id ")
            .push(direction)
            .push(" limit ")
            .push_bind(fetch_limit);

        let mut rows = builder
            .build_query_as::<FeedbackRow>()
            .fetch_all(&self.pool)
            .await?;

        let next_cursor = super::take_page(&mut rows, limit, |r| r.id);

        let mut feedbacks: Vec<Feedback> = Vec::with_capacity(rows.len());
        for row in rows {
            let feedback = row.try_into()?;
            feedbacks.push(feedback);
        }

        Ok((feedbacks, next_cursor))
    }

    #[instrument(name = "db.feedback.set_status", skip_all, fields(feedback.id = %item.id))]
    pub async fn set_feedback_status(
        &self,
        item: &FeedbackStatusUpdate,
    ) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!(
            "queries/set_feedback_status.sql",
            item.id,
            item.status.as_ref(),
            item.updated_at
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Feedback>());
        }

        Ok(())
    }
}
