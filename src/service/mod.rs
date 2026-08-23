mod book;
mod chapter;
mod content_rating;
mod creator;
mod label;
mod language;
mod release;

use crate::{database::Database, object_storage::ObjectStorage};

#[derive(Debug, Clone)]
pub struct Service {
    pub database: Database,
    pub covers: ObjectStorage,
    pub release_pages: ObjectStorage,
}

impl Service {
    pub fn new(database: Database, covers: ObjectStorage, release_pages: ObjectStorage) -> Self {
        Self {
            database,
            covers,
            release_pages,
        }
    }
}
