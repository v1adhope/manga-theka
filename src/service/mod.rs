mod book;
mod content_rating;
mod creator;
mod label;
mod language;

use crate::{database::Database, object_storage::Storage};

#[derive(Debug, Clone)]
pub struct Service {
    pub database: Database,
    pub covers: Storage,
}

impl Service {
    pub fn new(database: Database, covers: Storage) -> Self {
        Self { database, covers }
    }
}
