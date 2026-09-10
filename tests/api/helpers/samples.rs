#[derive(Debug)]
pub struct BookSample {
    pub name: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
    pub labels: i64,
    pub links: i64,
    pub titles: i64,
}

#[derive(Debug)]
pub struct BookVisibilityState {
    pub visibility: String,
    pub note: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
}

#[derive(Debug)]
pub struct ChapterSample {
    pub number: Option<f32>,
    pub name: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
    pub localizations: i64,
}
