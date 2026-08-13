use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{Label, LabelKind},
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
pub(super) struct LabelRow {
    pub(super) id: Uuid,
    pub(super) name: String,
    pub(super) kind: String,
}

impl TryFrom<LabelRow> for Label {
    type Error = DatabaseError;

    fn try_from(row: LabelRow) -> Result<Self, Self::Error> {
        let kind: LabelKind = row
            .kind
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("kind", e))?;

        Ok(Label {
            id: row.id,
            name: row.name,
            kind,
        })
    }
}

impl Database {
    #[tracing::instrument(name = "db.label.list", skip_all, fields(label.kind = ?label_kind))]
    pub async fn get_labels(
        &self,
        label_kind: Option<&LabelKind>,
    ) -> Result<Vec<Label>, DatabaseError> {
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("select id, name, kind from labels");

        if let Some(label_kind) = label_kind {
            builder
                .push(" where kind = ")
                .push_bind(label_kind.as_ref());
        }

        builder.push(" order by name");

        builder
            .build_query_as::<LabelRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .and_then(|rows| rows.into_iter().map(Label::try_from).collect())
            .inspect_err(DatabaseError::log_internal)
    }
}
