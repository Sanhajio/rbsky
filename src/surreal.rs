use anyhow::{Context, Result};
use atrium_api::app::bsky::feed;
use atrium_api::records::Record;
use chrono::{DateTime, ParseError, Utc};
use log::{error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::path::PathBuf;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::fs::create_dir_all;

use crate::runner::Runner;

#[derive(Clone)]
pub struct SurrealDB {
    db: Surreal<Db>,
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TimelineCursor {
    cursor: String,
    timeline: String,
}

// Implementation on TimelineCursor full ChatGPT, did not read them
// Optionally, implement `Ord` if you want total ordering and are sure every comparison will be valid
impl PartialEq for TimelineCursor {
    fn eq(&self, other: &Self) -> bool {
        self.timeline == other.timeline
            && parse_datetime(&self.cursor) == parse_datetime(&other.cursor)
    }
}

impl Eq for TimelineCursor {}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, ParseError> {
    s.parse::<DateTime<Utc>>()
}

// Since you might want to sort or store `TimelineCursor` in a hash set,
// you should ideally implement `Eq` and `PartialOrd` + `Ord` as well.

impl PartialOrd for TimelineCursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (parse_datetime(&self.cursor), parse_datetime(&other.cursor)) {
            (Ok(self_dt), Ok(other_dt)) => Some(self_dt.cmp(&other_dt)),
            _ => None,
        }
    }
}

impl Ord for TimelineCursor {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

fn compare_records_by_created_at(a: &Record, b: &Record) -> Ordering {
    let created_at_a = match a {
        Record::AppBskyFeedPost(post) => &post.created_at,
        _ => return Ordering::Equal, // Handle other cases if needed
    };

    let created_at_b = match b {
        Record::AppBskyFeedPost(post) => &post.created_at,
        _ => return Ordering::Equal, // Handle other cases if needed
    };

    created_at_b.cmp(&created_at_a) // Reverse order to sort in descending order
}

fn sort_timeline_by_created_at(timeline: &mut Vec<feed::defs::FeedViewPost>) {
    timeline.sort_by(|a, b| {
        let record_a = &a.post.record;
        let record_b = &b.post.record;
        compare_records_by_created_at(record_a, record_b)
    });
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

    pub async fn store_author(
        &self,
        author: atrium_api::app::bsky::actor::defs::ProfileViewDetailed,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("authors").await;
        let did: String = author.did.to_string().clone();
        let _created: Option<atrium_api::app::bsky::actor::defs::ProfileViewDetailed> =
            self.db.update(("author", did)).content(author).await?;
        Ok(())
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
        let mut timeline: Vec<feed::defs::FeedViewPost> =
            self.db.select(timeline_name.clone()).await?;
        sort_timeline_by_created_at(&mut timeline);

        info!(
            "Reading into {:?} timeline Db: {:?}",
            timeline_name,
            timeline.len()
        );

        Ok(timeline)
    }

    pub async fn read_cursor(
        &self,
        timeline_name: String,
    ) -> Result<Vec<TimelineCursor>, anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let cursor: Vec<TimelineCursor> = self.db.select("cursor").await?;
        info!("Reading into cursor timeline Db: {:?}", cursor);
        Ok(cursor)
    }

    pub async fn get_latest_cursor(
        &self,
        timeline_name: String,
    ) -> Result<Option<String>, anyhow::Error> {
        let cursors: Vec<TimelineCursor> = self.read_cursor(timeline_name.clone()).await?;
        info!("Reading cursors: {:?}", cursors);
        let max = cursors.into_iter().max();
        match max {
            Some(m) => {
                info!("max cursor: {:?}", m);
                Ok(Some(m.cursor))
            }
            _ => {
                error!("No cursors found");
                Ok(None)
            }
        }
    }
}
