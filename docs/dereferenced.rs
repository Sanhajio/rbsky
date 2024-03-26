// I am giving up dereferencing the feedviewpost as it looks way too hard
use anyhow::{Context, Result};
use atrium_api::app::bsky;
use atrium_api::app::bsky::feed;
use atrium_api::app::bsky::feed::defs::{FeedViewPost, PostView};
use atrium_api::records::Record;
use chrono::{DateTime, ParseError, Utc};
use log::{error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::fs::create_dir_all;

#[derive(Serialize, Deserialize, Debug)]
struct TimelineFeed {
    id: String,
    post: FeedViewPost,
    reason: Option<feed::defs::FeedViewPostReasonEnum>,
    reply: Option<Reply>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Reply {
    parent: Option<feed::defs::FeedViewPost>,
    root: Option<feed::defs::FeedViewPost>,
}

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

#[derive(Serialize, Deserialize, Debug)]
pub struct TimelineResponse {}

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

    pub async fn store_post(
        &self,
        post: atrium_api::app::bsky::feed::defs::PostView,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let cid: String = serde_json::to_string(&post.cid.clone())?
            .trim_matches('"')
            .to_string();
        let did: String = post.author.did.to_string().clone();
        // let _created: Option<atrium_api::app::bsky::feed::defs::PostView> =
        //     self.db.update(("post", cid)).content(post).await?;
        let sql = format!(
            r#"UPDATE post:{} CONTENT {{
                author: author:⟨{}⟩,
                indexedAt: {},
                labels: {},
                likeCount: {},
                record: {},
                replyCount: {},
                repostCount: {},
                uri: {},
                viewer: {},
        }};"#,
            cid,
            did,
            serde_json::to_string(&post.indexed_at)?,
            serde_json::to_string(&post.labels)?,
            serde_json::to_string(&post.like_count)?,
            serde_json::to_string(&post.record)?,
            serde_json::to_string(&post.reply_count)?,
            serde_json::to_string(&post.repost_count)?,
            serde_json::to_string(&post.uri)?,
            serde_json::to_string(&post.viewer)?,
        );
        info!("{}", sql);
        let _created = self.db.query(sql).await?;
        Ok(())
    }

    pub async fn store_author(
        &self,
        author: atrium_api::app::bsky::actor::defs::ProfileViewBasic,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let did: String = author.did.to_string().clone();
        let _created: Option<atrium_api::app::bsky::actor::defs::ProfileViewBasic> = self
            .db
            .update(("author", did.clone()))
            .content(author)
            .await?;
        if let Some(created) = _created {
            trace!("Inserting into author table {:?}", created);
        } else {
            trace!("unable to create entry {}", did.clone());
        }
        Ok(())
    }

    pub async fn store_post_view(
        &self,
        post: atrium_api::app::bsky::feed::defs::PostView,
    ) -> Result<(), anyhow::Error> {
        let author: bsky::actor::defs::ProfileViewBasic = post.author.clone();
        self.store_author(author).await?;
        self.store_post(post).await?;
        Ok(())
    }

    pub async fn store_feed_post_view(
        &self,
        f: atrium_api::app::bsky::feed::defs::FeedViewPost,
    ) -> Result<(), anyhow::Error> {
        let cid: String = serde_json::to_string(&f.post.cid.clone())?
            .trim_matches('"')
            .to_string();
        self.store_post_view(f.post).await?;

        let mut cid_parent: Option<String> = None;
        let mut cid_root: Option<String> = None;
        if let Some(reply) = f.reply {
            match reply.parent {
                feed::defs::ReplyRefParentEnum::PostView(parent) => {
                    cid_parent = Some(
                        serde_json::to_string(&parent.cid.clone())?
                            .trim_matches('"')
                            .to_string(),
                    );
                    self.store_post_view(*parent).await?;
                }
                feed::defs::ReplyRefParentEnum::BlockedPost(_parent) => {}
                feed::defs::ReplyRefParentEnum::NotFoundPost(_parent) => {}
            }
            match reply.root {
                feed::defs::ReplyRefRootEnum::PostView(root) => {
                    cid_root = Some(
                        serde_json::to_string(&root.cid.clone())?
                            .trim_matches('"')
                            .to_string(),
                    );
                    self.store_post_view(*root).await?;
                }
                feed::defs::ReplyRefRootEnum::NotFoundPost(_root) => {}
                feed::defs::ReplyRefRootEnum::BlockedPost(_root) => {}
            }
        }
        if let Some(reason) = f.reason.clone() {
            match reason {
                feed::defs::FeedViewPostReasonEnum::ReasonRepost(reason) => {}
            }
        }
        match (cid_root, cid_parent) {
            (Some(root), Some(parent)) => {
                let sql = format!(
                    r#"UPDATE feed:{} CONTENT {{
                        post: post:{},
                        reply: {{
                          parent: feed:⟨{}⟩,
                          root: feed:⟨{}⟩,
                        }},
                        reason: {},
                }};"#,
                    cid,
                    cid,
                    serde_json::to_string(&parent)?
                        .trim_matches('"')
                        .to_string(),
                    serde_json::to_string(&root)?.trim_matches('"').to_string(),
                    serde_json::to_string(&f.reason)?,
                );
                info!("{}", sql);
                let _created = self.db.query(sql).await?;
            }
            _ => {
                let sql = format!(
                    r#"UPDATE feed:{} CONTENT {{
                        post: post:{},
                        reason: {},
                }};"#,
                    cid,
                    cid,
                    serde_json::to_string(&f.reason)?,
                );
                info!("{}", sql);
                let _created = self.db.query(sql).await?;
            }
        }
        Ok(())
    }

    pub async fn store_feed_post_raw(
        &self,
        feed: Vec<atrium_api::app::bsky::feed::defs::FeedViewPost>,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        for f in feed {
            let cid: String = serde_json::to_string(&f.post.cid.clone())?
                .trim_matches('"')
                .to_string();
            let _created: Option<atrium_api::app::bsky::feed::defs::FeedViewPost> =
                self.db.update(("feedviewpost", cid)).content(f).await?;
        }

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
            self.store_feed_post_view(f).await?;
        }
        self.store_cursor(timeline_data.cursor, timeline_name)
            .await?;
        Ok(())
    }

    pub async fn store_cursor(
        &self,
        cursor: Option<String>,
        timeline_name: String,
    ) -> Result<(), anyhow::Error> {
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
        let mut timeline: Vec<feed::defs::FeedViewPost> = self.db.select("feed").await?;
        sort_timeline_by_created_at(&mut timeline);

        info!(
            "Reading into {:?} timeline Db: {:?}",
            timeline_name,
            timeline.len()
        );

        Ok(timeline)
    }

    pub async fn read_timeline_raw(
        &self,
        timeline_name: String,
    ) -> Result<Vec<feed::defs::FeedViewPost>, anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let timeline: Vec<feed::defs::FeedViewPost> = self.db.select("feedviewpost").await?;
        info!(
            "Reading into {:?} timeline Db: {:?}",
            timeline_name, timeline
        );
        Ok(timeline)
    }

    pub async fn read_timeline_raw_query(
        &self,
        timeline_name: String,
    ) -> Result<(), anyhow::Error> {
        let _ = self.db.use_ns("bsky").use_db("timeline").await;
        let mut result = self.db.query(r#"SELECT id FROM feed LIMIT 1;"#).await?;
        info!("value is {:?}", result);
        let value: Vec<serde_json::Value> = result.take(0)?;
        for f in value {
            // let desered: TimelineFeed = serde_json::from_value(f.clone())?;
            // info!("desered is {:?}", desered);
            // info!("value is {}", serde_json::to_string_pretty(&f)?);
            info!("value is {:?}", f);
        }
        // let json: Vec<FeedViewPost> = serde_json::from_value(value_as_array)?;
        Ok(())
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
