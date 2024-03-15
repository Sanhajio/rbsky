use anyhow::{Context, Result};
use atrium_api::app::bsky::feed;
use log::{trace, warn};
use std::path::PathBuf;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::fs::create_dir_all;

pub struct SurrealDB {
    db: Surreal<Db>,
    path: PathBuf,
}

// TODO: maybe change this to BSKYDB
impl SurrealDB {
    pub async fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .with_context(|| format!("No config dir: {:?}", dirs::config_dir()))?;
        let dir = config_dir.join("bsky");
        create_dir_all(&dir).await?;
        let path = dir.join("bsky.db");
        let db = Surreal::new::<RocksDb>(path.clone()).await?;
        Ok(SurrealDB { db, path })
    }

    pub async fn store_timeline(
        &self,
        timeline: feed::get_timeline::Output,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let feed: Vec<feed::defs::FeedViewPost> = timeline.feed;
        let cursor: Option<String> = timeline.cursor;
        let created: Vec<feed::defs::FeedViewPost> =
            self.db.create("default").content(feed).await?;
        trace!("Inserted in DB: {:?}", created);
        match cursor {
            Some(cursor) => {
                let created: Vec<String> = self.db.create("cursor").content(cursor).await?;
                trace!("Inserted in DB: {:?}", created);
            }
            None => warn!("cursor is none"),
        }
        Ok(())
    }
}
