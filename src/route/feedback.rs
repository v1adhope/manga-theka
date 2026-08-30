use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{
        Email, Feedback, FeedbackFilter, FeedbackKind, FeedbackStatus, FeedbackStatusUpdate,
        Filter, SortOrder, Text,
    },
    error::{AppError, EntityError},
    route::{StoreResp, json_data_response, json_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum FeedbackReq {
    Report {
        email: String,
        note: String,
        book_id: Uuid,
    },
    Correction {
        email: String,
        note: String,
        book_id: Uuid,
    },
    General {
        email: String,
        note: String,
    },
}

impl TryFrom<(FeedbackReq, Uuid, OffsetDateTime)> for Feedback {
    type Error = EntityError;

    fn try_from(ctx: (FeedbackReq, Uuid, OffsetDateTime)) -> Result<Self, Self::Error> {
        let (req, id, created_at) = ctx;

        let (kind, email, note, book_id) = match req {
            FeedbackReq::Report {
                email,
                note,
                book_id,
            } => (FeedbackKind::Report, email, note, Some(book_id)),
            FeedbackReq::Correction {
                email,
                note,
                book_id,
            } => (FeedbackKind::Correction, email, note, Some(book_id)),
            FeedbackReq::General { email, note } => (FeedbackKind::General, email, note, None),
        };

        Ok(Self {
            id,
            kind,
            status: FeedbackStatus::Open,
            email: Email::try_from(email)?,
            note: Text::try_from(note)?,
            book_id,
            updated_at: None,
            created_at,
        })
    }
}

pub async fn store_feedback(
    State(service): State<Service>,
    Json(req): Json<FeedbackReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = OffsetDateTime::now_utc();
    let feedback: Feedback = (req, id, created_at).try_into()?;

    service.store_feedback(feedback).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackListQuery {
    pub kind: Option<FeedbackKind>,
    pub status: Option<FeedbackStatus>,
    pub order: Option<SortOrder>,
    pub after: Option<Uuid>,
    pub limit: Option<u32>,
}

impl TryFrom<FeedbackListQuery> for FeedbackFilter {
    type Error = EntityError;

    fn try_from(q: FeedbackListQuery) -> Result<Self, Self::Error> {
        let page = Filter::builder()
            .after(q.after)
            .limit(q.limit)
            .sort_order(q.order)
            .build()?;

        Ok(Self {
            page,
            kind: q.kind,
            status: q.status,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFeedbacksResp {
    pub data: Vec<Feedback>,
    pub next_cursor: Option<Uuid>,
}

// deferred: gate to Moderator/Admin
pub async fn get_feedbacks(
    State(service): State<Service>,
    Query(query): Query<FeedbackListQuery>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let filter: FeedbackFilter = query.try_into()?;

    let (data, next_cursor) = service.get_feedbacks(filter).await?;

    Ok(json_response(
        StatusCode::OK,
        GetFeedbacksResp { data, next_cursor },
    ))
}

// deferred: gate to Moderator/Admin
pub async fn get_feedback(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let feedback = service.get_feedback(id).await?;
    Ok(json_data_response(StatusCode::OK, feedback))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackStatusReq {
    pub status: FeedbackStatus,
}

impl From<(FeedbackStatusReq, Uuid, OffsetDateTime)> for FeedbackStatusUpdate {
    fn from(ctx: (FeedbackStatusReq, Uuid, OffsetDateTime)) -> Self {
        let (req, id, updated_at) = ctx;

        Self {
            id,
            status: req.status,
            updated_at,
        }
    }
}

// deferred: gate to Moderator/Admin
pub async fn update_feedback_status(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<FeedbackStatusReq>,
) -> Result<StatusCode, AppError> {
    let update: FeedbackStatusUpdate = (req, id, OffsetDateTime::now_utc()).into();

    service.set_feedback_status(update).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::{
        entity::{Feedback, FeedbackKind, FeedbackStatus},
        route::FeedbackReq,
    };

    #[test]
    fn every_kind_parses_to_its_variant() {
        let book_id = Uuid::now_v7();

        for (kind, body) in [
            (
                FeedbackKind::Report,
                serde_json::json!({"kind": "Report", "email": "a@b.co", "note": "n", "bookId": book_id}),
            ),
            (
                FeedbackKind::Correction,
                serde_json::json!({"kind": "Correction", "email": "a@b.co", "note": "n", "bookId": book_id}),
            ),
            (
                FeedbackKind::General,
                serde_json::json!({"kind": "General", "email": "a@b.co", "note": "n"}),
            ),
        ] {
            let req: FeedbackReq = serde_json::from_value(body).expect("body must parse");
            let feedback = Feedback::try_from((req, Uuid::now_v7(), OffsetDateTime::now_utc()))
                .expect("feedback must build");

            assert_eq!(feedback.kind, kind);
        }
    }

    #[test]
    fn general_feedback_naming_a_book_is_rejected() {
        let res = serde_json::from_value::<FeedbackReq>(serde_json::json!({
            "kind": "General",
            "email": "a@b.co",
            "note": "n",
            "bookId": Uuid::now_v7(),
        }));

        assert!(res.is_err());
    }

    #[test]
    fn book_bound_feedback_without_a_book_is_rejected() {
        for kind in ["Report", "Correction"] {
            let res = serde_json::from_value::<FeedbackReq>(serde_json::json!({
                "kind": kind,
                "email": "a@b.co",
                "note": "n",
            }));

            assert!(res.is_err(), "{kind} without a bookId must be rejected");
        }
    }

    #[test]
    fn book_bound_feedback_carries_its_book() {
        let book_id = Uuid::now_v7();
        let req: FeedbackReq = serde_json::from_value(serde_json::json!({
            "kind": "Report", "email": "a@b.co", "note": "n", "bookId": book_id,
        }))
        .unwrap();

        let feedback =
            Feedback::try_from((req, Uuid::now_v7(), OffsetDateTime::now_utc())).unwrap();

        assert_eq!(feedback.book_id, Some(book_id));
    }

    #[test]
    fn general_feedback_carries_no_book() {
        let req: FeedbackReq = serde_json::from_value(serde_json::json!({
            "kind": "General", "email": "a@b.co", "note": "n",
        }))
        .unwrap();

        let feedback =
            Feedback::try_from((req, Uuid::now_v7(), OffsetDateTime::now_utc())).unwrap();

        assert_eq!(feedback.book_id, None);
    }

    #[test]
    fn server_owned_fields_come_from_the_context_not_the_body() {
        let id = Uuid::now_v7();
        let created_at = OffsetDateTime::now_utc();
        let req: FeedbackReq = serde_json::from_value(serde_json::json!({
            "kind": "General", "email": "a@b.co", "note": "n",
        }))
        .unwrap();

        let feedback = Feedback::try_from((req, id, created_at)).unwrap();

        assert_eq!(feedback.id, id);
        assert_eq!(feedback.created_at, created_at);
        assert_eq!(feedback.status, FeedbackStatus::Open);
        assert_eq!(feedback.updated_at, None);
    }

    #[test]
    fn invalid_field_values_are_rejected() {
        for (label, body) in [
            (
                "malformed email",
                serde_json::json!({"kind": "General", "email": "nope", "note": "n"}),
            ),
            (
                "empty note",
                serde_json::json!({"kind": "General", "email": "a@b.co", "note": "  "}),
            ),
            (
                "oversized note",
                serde_json::json!({"kind": "General", "email": "a@b.co", "note": "e".repeat(2001)}),
            ),
        ] {
            let req: FeedbackReq = serde_json::from_value(body).expect("body must parse");
            let res = Feedback::try_from((req, Uuid::now_v7(), OffsetDateTime::now_utc()));

            assert!(res.is_err(), "{label} must be rejected");
        }
    }
}
