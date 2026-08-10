use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{Label, LabelKind},
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
struct LabelRow {
    id: Uuid,
    name: String,
    kind: String,
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
    pub async fn get_labels(
        &self,
        label_kind: Option<LabelKind>,
    ) -> Result<Vec<Label>, DatabaseError> {
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("select id, name, kind from labels");

        if let Some(label_kind) = label_kind {
            builder
                .push(" where kind = ")
                .push_bind(label_kind.as_ref());
        }

        builder.push(" order by name");

        let rows = builder
            .build_query_as::<LabelRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get labels from database: {e:?}");
                }
            })?;

        let mut labels: Vec<Label> = Vec::with_capacity(rows.len());
        for row in rows {
            let label = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert label row: {e:?}");
                }
            })?;
            labels.push(label);
        }

        Ok(labels)
    }
}
