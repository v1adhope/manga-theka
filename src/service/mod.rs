mod content_rating;
mod creator;
mod label;
mod language;

use crate::database::Database;

#[derive(Debug, Clone)]
pub struct Service {
    pub database: Database,
}

impl Service {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}
