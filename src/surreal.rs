use std::path::Path;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::{Result, Surreal};

pub struct SurrealDB {
    db: Surreal<Db>,
}

impl SurrealDB {
    pub async fn new(path: &Path) -> Result<Self> {
        let _db = Surreal::new::<RocksDb>(path).await?;
        Ok(SurrealDB { db: _db })
    }
}
