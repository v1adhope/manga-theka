mod book;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod label;
mod language;
mod release;

use crate::{database::Database, object_storage::ObjectStorage};

#[derive(Debug, Clone)]
pub struct Service {
    pub database: Database,
    pub storage: ObjectStorage,
}

impl Service {
    pub fn new(database: Database, storage: ObjectStorage) -> Self {
        Self { database, storage }
    }
}
