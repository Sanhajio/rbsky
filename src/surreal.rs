use anyhow::{Context, Result};
use atrium_api::app::bsky::feed;
use atrium_api::app::bsky::feed::defs::{FeedViewPost, PostView};
use log::{error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::fs::create_dir_all;

pub struct SurrealDB {
    db: Surreal<Db>,
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TimelineCursor {
    cursor: String,
    timeline: String,
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
        timeline_data: feed::get_timeline::Output,
        timeline_name: String,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let feed: Vec<feed::defs::FeedViewPost> = timeline_data.feed;
        info!(
            "Inserting into {:?} timeline Db: {:?}",
            timeline_name,
            feed.len()
        );

        for f in feed {
            let fj: Value = serde_json::to_value(f)?.clone();
            info!("Inserting into {:?} timeline Db: {:?}", timeline_name, fj);

            let _created: Vec<Value> = self.db.create("default").content(fj).await?;
        }

        let cursor: Option<String> = timeline_data.cursor;
        match cursor {
            Some(cursor) => {
                trace!("Inserting: {:?}", cursor);
                let created: Vec<TimelineCursor> = self
                    .db
                    .create("cursor")
                    .content(TimelineCursor {
                        cursor: String::from(cursor),
                        timeline: timeline_name,
                    })
                    .await?;
                trace!("Inserted in DB: {:?}", created);
            }
            None => warn!("cursor is none"),
        }
        Ok(())
    }

    pub async fn read_timeline(
        &self,
        timeline_name: String,
    ) -> Result<Vec<feed::defs::FeedViewPost>, anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let feed: Vec<feed::defs::FeedViewPost> = self.db.select(timeline_name.clone()).await?;
        info!(
            "Reading into {:?} timeline Db: {:?}",
            timeline_name,
            feed.len()
        );
        Ok(feed)
    }
}
