mod book;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod label;
mod language;
mod release;
mod session;
mod user;
mod visibility;

use crate::{
    database::Database, hasher::Hasher, jwt::Jwt, memory_storage::MemoryStore,
    object_storage::ObjectStorage,
};

#[derive(Debug, Clone)]
pub struct Service {
    pub database: Database,
    pub storage: ObjectStorage,
    pub hasher: Hasher,
    pub jwt: Jwt,
    pub memory: MemoryStore,
}

pub use session::SessionTokens;

impl Service {
    pub fn new(
        database: Database,
        storage: ObjectStorage,
        hasher: Hasher,
        jwt: Jwt,
        memory: MemoryStore,
    ) -> Self {
        Self {
            database,
            storage,
            hasher,
            jwt,
            memory,
        }
    }
}
