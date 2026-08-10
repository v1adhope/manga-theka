use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::{error::AppError, route::json_data_response, service::Service};

pub async fn get_languages(
    State(service): State<Service>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let languages = service.get_languages().await?;
    Ok(json_data_response(StatusCode::OK, languages))
}
