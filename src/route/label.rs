use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{error::AppError, route::json_data_response, service::Service};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLabelsQuery {
    pub kind: Option<String>,
}

pub async fn get_labels(
    State(service): State<Service>,
    Query(query): Query<GetLabelsQuery>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let label_kind = query.kind.map(|t| t.parse()).transpose()?;
    let labels = service.get_labels(label_kind).await?;
    Ok(json_data_response(StatusCode::OK, labels))
}
