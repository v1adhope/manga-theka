use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{Label, LabelType},
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
struct LabelRow {
    id: Uuid,
    name: String,
    r#type: String,
}

impl TryFrom<LabelRow> for Label {
    type Error = DatabaseError;

    fn try_from(row: LabelRow) -> Result<Self, Self::Error> {
        let r#type: LabelType = row
            .r#type
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("type", e))?;

        Ok(Label {
            id: row.id,
            name: row.name,
            r#type,
        })
    }
}

impl Database {
    pub async fn get_labels(
        &self,
        label_type: Option<LabelType>,
    ) -> Result<Vec<Label>, DatabaseError> {
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("select id, name, type from labels");

        if let Some(label_type) = label_type {
            builder
                .push(" where type = ")
                .push_bind(label_type.as_ref());
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
