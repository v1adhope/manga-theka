use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::{error::AppError, route::json_data_response, service::Service};

pub async fn get_content_ratings(
    State(service): State<Service>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let content_ratings = service.get_content_ratings().await?;
    Ok(json_data_response(StatusCode::OK, content_ratings))
}
