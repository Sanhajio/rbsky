use std::collections::HashMap;
use std::sync::Arc;

use crate::sql::Querier;
use crate::surreal::SurrealDB;
use crate::{commands::GetTimelineArgs, runner::Runner};
use atrium_api::app::bsky::feed::defs::PostView;
use futures::lock::Mutex;
use log::{error, info, trace};
use neovim_lib::{Neovim, RequestHandler, Session};
use serde::{Deserialize, Serialize};
use tokio::time::{self, Duration};

enum Messages {
    Read,
    Update,
    Post,
    RePost,
    Like,
    UnLike,
    FetchMore,
    Refresh,
    Unknown(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeedViewPostFlat {
    pub post: PostView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PostView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<PostView>,
}

impl PartialEq for FeedViewPostFlat {
    fn eq(&self, other: &Self) -> bool {
        self.post.cid == other.post.cid
    }
}

impl Eq for FeedViewPostFlat {}

pub struct BskyRequestHandler {
    pub feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
}

impl RequestHandler for BskyRequestHandler {
    fn handle_request(
        &mut self,
        name: &str,
        args: Vec<neovim_lib::Value>,
    ) -> Result<neovim_lib::Value, neovim_lib::Value> {
        trace!("Received rpcrequest name: {:?}, args: {:?}", name, args);
        match Messages::from(name) {
            Messages::Read => Ok(self.handle_read_request()),
            Messages::FetchMore => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::Refresh => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::Post => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::Update => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::RePost => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::Like => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::UnLike => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
            Messages::Unknown(_event) => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
        }
    }
}

impl BskyRequestHandler {
    pub fn handle_read_request(&mut self) -> neovim_lib::Value {
        info!("request_handler: acquiring feed lock");
        let locked = self.feed.lock();
        match locked {
            Ok(l) => {
                let opt = l.clone();
                match opt {
                    Some(f) => {
                        drop(l);
                        let feed_len = f.len();
                        info!("request_handler returning: {feed_len} data");
                        let feed_json = serde_json::to_string(&f);
                        info!("request_handler: lock dropped");
                        match feed_json {
                            Ok(s) => {
                                trace!("Read Handler has value: {:?}", s);
                                return neovim_lib::Value::from(s.as_str());
                            }
                            Err(e) => {
                                error!("Error deserializing the feed: returning nil: {e}");
                                return neovim_lib::Value::from("nil");
                            }
                        }
                    }
                    None => {
                        drop(l);
                        error!("Lock acquired: No data in opt");
                        return neovim_lib::Value::from("nil");
                    }
                }
            }
            Err(_) => {
                error!("Unable to acquire the lock: returning nil");
                return neovim_lib::Value::from("nil");
            }
        }
    }
}

pub struct EventHandler {
    pub nvim: Neovim,
    pub db: Arc<Mutex<SurrealDB>>,
    pub runner: Runner,
}

impl EventHandler {
    pub fn new(db: Arc<Mutex<SurrealDB>>, runner: Runner) -> Result<EventHandler, anyhow::Error> {
        let session = Session::new_parent()?;
        let nvim = Neovim::new(session);
        let db = db;
        Ok(EventHandler { nvim, db, runner })
    }

    // TODO: add args to the recv function, add timeline, which timeline should I read
    // Add this in the setup of the binary adding clap
    pub async fn recv(
        &mut self,
        bsky_request_handler: BskyRequestHandler,
    ) -> Result<(), anyhow::Error> {
        let feed = Arc::clone(&bsky_request_handler.feed);
        let receiver = self
            .nvim
            .session
            .start_event_loop_channel_handler(bsky_request_handler);
        self.update_timeline(None).await?;
        self.update_feed(feed.clone()).await?;
        for (event, values) in receiver {
            trace!("Received rpcevent: {:?}, values: {:?}", event, values);
            match Messages::from(event) {
                Messages::Read => {
                    error!("Uninmplemented");
                }
                Messages::Post => {
                    error!("Uninmplemented");
                }
                Messages::Update => {
                    // args: values[0] contains the first cid from that neovim sends
                    if values.is_empty() {
                        let now = chrono::offset::Local::now().to_rfc3339();
                        self.update_timeline(Some(now)).await?;
                    }
                    self.clean_feed(feed.clone()).await?;
                    self.update_feed(feed.clone()).await?;
                }
                Messages::RePost => {
                    error!("Uninmplemented");
                }
                Messages::Like => {
                    error!("Uninmplemented");
                }
                Messages::UnLike => {
                    error!("Uninmplemented");
                }
                Messages::FetchMore => {
                    // args: values[0] contains the last cid from that neovim sends
                    self.fetch_more(values[0].to_string(), feed.clone()).await?;
                }
                Messages::Refresh => {
                    // args: values[0] contains the first cid from that neovim sends
                    if values.is_empty() {
                        self.update_timeline(None).await?;
                    }
                    self.update_feed(feed.clone()).await?;
                }
                Messages::Unknown(event) => {
                    error!("Uninmplemented {}", event);
                }
            }
        }
        Ok(())
    }

    async fn clean_feed(
        &mut self,
        feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
    ) -> Result<(), anyhow::Error> {
        let locked = feed.lock();
        let empty: Vec<FeedViewPostFlat> = vec![];
        match locked {
            Ok(mut l) => {
                info!("nvim_feed_lock Acquired Lock Cleaning Data");
                *l = Some(empty);
                info!("nvim_feed_lock Droping Lock");
                drop(l);
                return Ok(());
            }
            Err(e) => {
                error!("error while fetching data {:?}", e);
            }
        };
        Ok(())
    }

    async fn merge_feed(
        feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
        more: Vec<FeedViewPostFlat>,
    ) -> Result<(), anyhow::Error> {
        let locked = feed.lock();
        match locked {
            Ok(mut l) => {
                info!("nvim_feed_lock Acquired Lock Updating Data");
                if let Some(ref mut existing_feed) = *l {
                    for item in more.clone() {
                        if !existing_feed.contains(&item) {
                            existing_feed.push(item);
                        }
                    }
                } else {
                    *l = Some(more.clone());
                }
                info!("nvim_feed_lock Droping Lock");
                drop(l);
                return Ok(());
            }
            Err(e) => {
                error!("error while fetching data {:?}", e);
            }
        };
        Ok(())
    }

    pub async fn refresh_from_cid(&mut self, cid: String) -> Result<(), anyhow::Error> {
        info!("refresh from cid {cid} feed");
        let db_lock = self.db.lock().await;
        let db = &db_lock.db;
        let _ = db.use_ns("bsky").use_db("timeline").await;

        let cid_created_at: String = Querier::new(db.clone())
            .select_created_at(cid.as_str())
            .await?;

        let new_cursor_time = chrono::DateTime::parse_from_rfc3339(&cid_created_at)?
            .checked_add_signed(
                chrono::Duration::try_minutes(10).expect("Unable to convert to minutes"),
            )
            .expect("Time calculation error")
            .to_rfc3339();

        info!(
            "fetching timeline with new cursor at: {:?}",
            new_cursor_time
        );

        let timeline = self
            .runner
            ._get_timeline(GetTimelineArgs {
                algorithm: String::from("reverse-chronological"),
                cursor: Some(new_cursor_time),
                limit: 10,
            })
            .await;

        let count_newer_than: i32 = Querier::new(db.clone())
            .count_posts_newer_than(cid_created_at.as_str())
            .await?;

        info!(
            "cid: {}, date: {}, count newer than: {}",
            cid.as_str(),
            cid_created_at,
            count_newer_than,
        );

        match timeline {
            Ok(data) => {
                trace!("read timeline {:?}", data);
                let write_res = db_lock
                    .store_timeline(data.clone(), String::from("default"))
                    .await;
                match write_res {
                    Ok(res) => {
                        info!("data written successfully: {:?}", res)
                    }
                    Err(e) => error!("error while fetching data {:?}", e),
                }
            }
            Err(e) => {
                error!("error while fetching data {:?}", e);
            }
        }
        drop(db_lock);
        Ok(())
    }

    pub async fn fetch_more(
        &mut self,
        cid: String,
        feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
    ) -> Result<(), anyhow::Error> {
        info!("fetch more data into the feed handler");
        let db_lock = self.db.lock().await;
        let db = &db_lock.db;
        let _ = db.use_ns("bsky").use_db("timeline").await;

        let cid_created_at: String = Querier::new(db.clone())
            .select_created_at(cid.as_str())
            .await?;
        let mut result_number: i32 = 0;
        let mut cursor: String = cid_created_at.clone();

        while result_number < 11 {
            let lower_limit = chrono::DateTime::parse_from_rfc3339(cid_created_at.as_str())?
                .checked_sub_signed(
                    chrono::Duration::try_minutes(300).expect("Unable to convert to minutes"),
                )
                .expect("Time calculation error")
                .to_rfc3339();

            let count_recent_older_than: i32 = Querier::new(db.clone())
                .count_recent_posts_older_than(cid_created_at.as_str(), lower_limit.as_str())
                .await?;
            result_number = count_recent_older_than;
            info!(
                "cid: {}, date: {}, count older than: {}; result number: {} ",
                cid.as_str(),
                count_recent_older_than,
                result_number,
                cursor
            );
            if count_recent_older_than > 11 {
                let lower_limit = chrono::DateTime::parse_from_rfc3339(&cursor)?
                    .checked_sub_signed(
                        chrono::Duration::try_minutes(300).expect("Unable to convert to minutes"),
                    )
                    .expect("Time calculation error")
                    .to_rfc3339();

                let more: Vec<FeedViewPostFlat> = db_lock
                    .read_timeline(
                        String::from("default"),
                        Some(format!(
                            "createdAt < '{cursor}' and createdAt >= '{lower_limit}'"
                        )),
                    )
                    .await?;
                // EventHandler::merge_feed(feed.clone(), more).await?;
                let locked = feed.lock();
                match locked {
                    Ok(mut l) => {
                        info!("nvim_feed_lock Acquired Lock Updating Data");
                        if let Some(ref mut existing_feed) = *l {
                            let existing_feed_len = existing_feed.len();
                            let more_feed_len = more.len();
                            info!("merging existing feed: {existing_feed_len} with more items {more_feed_len}");
                            for item in more.clone() {
                                if !existing_feed.contains(&item) {
                                    existing_feed.push(item);
                                }
                            }
                            let existing_feed_len = existing_feed.len();
                            info!("existing feed merged: {existing_feed_len}");
                        } else {
                            *l = Some(more.clone());
                        }
                        info!("nvim_feed_lock Droping Lock");
                        drop(l);
                        return Ok(());
                    }
                    Err(e) => {
                        error!("error while fetching data {:?}", e);
                    }
                };
            } else {
                let new_cursor_time = chrono::DateTime::parse_from_rfc3339(&cursor)?
                    .checked_sub_signed(
                        chrono::Duration::try_minutes(30).expect("Unable to convert to minutes"),
                    )
                    .expect("Time calculation error")
                    .to_rfc3339();
                info!(
                    "fetching timeline with new cursor at: {:?}",
                    new_cursor_time
                );
                cursor = new_cursor_time.clone();
                let timeline = self
                    .runner
                    ._get_timeline(GetTimelineArgs {
                        algorithm: String::from("reverse-chronological"),
                        cursor: Some(cursor.to_string()),
                        limit: 10,
                    })
                    .await;
                match timeline {
                    Ok(data) => {
                        trace!("read timeline {:?}", data);
                        let write_res = db_lock
                            .store_timeline(data.clone(), String::from("default"))
                            .await;
                        match write_res {
                            Ok(res) => {
                                info!("data written successfully: {:?}", res)
                            }
                            Err(e) => error!("error while fetching data {:?}", e),
                        }
                    }
                    Err(e) => {
                        error!("error while fetching data {:?}", e);
                    }
                }
            }
        }
        drop(db_lock);
        Ok(())
    }

    // This function updates the feed that is sent back to neovim
    pub async fn update_feed(
        &mut self,
        feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
    ) -> Result<(), anyhow::Error> {
        trace!("updating read handler feed");
        let db_lock = self.db.lock().await;
        let cached_feed: Vec<FeedViewPostFlat> =
            db_lock.read_timeline(String::from("default"), None).await?;
        trace!("reading the data: {:?}", cached_feed);
        let locked = feed.lock();
        match locked {
            Ok(mut l) => {
                *l = Some(cached_feed);
                drop(l);
                return Ok(());
            }
            Err(e) => {
                error!("error while fetching data {:?}", e);
            }
        };
        drop(db_lock);
        Ok(())
    }

    // This function updates the timeline in the db
    pub async fn update_timeline(
        &mut self,
        mut cursor: Option<String>,
    ) -> Result<(), anyhow::Error> {
        let db_lock = self.db.lock().await;
        // TODO: This leaves the timeline stuck at a point in time
        // I wanted to get all the data from latest_cursor to now basically
        // But It's not yet handled well
        match cursor {
            Some(ref _c) => {}
            /*
            None => {
                let cursor_res = db_lock.get_latest_cursor(String::from("default")).await;
                match cursor_res {
                    Ok(res) => {
                        info!("found cursor at: {:?}", res);
                        cursor = res;
                    }
                    Err(e) => error!("error while fetching cursor data {:?}", e),
                }
            }
            */
            None => {}
        }

        let timeline = self
            .runner
            ._get_timeline(GetTimelineArgs {
                algorithm: String::from("reverse-chronological"),
                cursor,
                limit: 10,
            })
            .await;
        match timeline {
            Ok(data) => {
                trace!("read timeline {:?}", data);
                let write_res = db_lock
                    .store_timeline(data.clone(), String::from("default"))
                    .await;
                match write_res {
                    Ok(res) => info!("data written successfully: {:?}", res),
                    Err(e) => error!("error while fetching data {:?}", e),
                }
            }
            Err(e) => {
                error!("error while fetching data {:?}", e);
            }
        }
        drop(db_lock);
        Ok(())
    }

    pub async fn auto_refresh_timeline(
        &mut self,
        task_interval: Duration,
        nvim_feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
    ) -> Result<(), anyhow::Error> {
        let mut interval = time::interval(task_interval);
        loop {
            interval.tick().await;
            trace!("executed background task");
            self.update_timeline(None).await?;
            let db_lock = self.db.lock().await;
            let data: Vec<FeedViewPostFlat> =
                db_lock.read_timeline(String::from("default"), None).await?;
            let nvim_feed_lock = nvim_feed.lock();
            match nvim_feed_lock {
                Ok(mut l) => {
                    info!("nvim_feed_lock Acquired Lock Updating Data");
                    if let Some(ref mut existing_feed) = *l {
                        // TODO: This is really ugly, but let's take care of it later
                        for item in data {
                            if !existing_feed.contains(&item) {
                                existing_feed.push(item);
                            }
                        }
                    } else {
                        *l = Some(data.clone());
                    }
                    info!("nvim_feed_lock Droping Lock");
                    drop(l);
                }
                Err(_) => error!("Unable to aquire the lock"),
            }
        }
    }
}

impl From<&str> for Messages {
    fn from(event: &str) -> Self {
        match event {
            "read" => Messages::Read,
            "post" => Messages::Post,
            "update" => Messages::Update,
            "repost" => Messages::RePost,
            "like" => Messages::Like,
            "more" => Messages::FetchMore,
            "refresh" => Messages::Refresh,
            _ => Messages::Unknown(event.to_string()),
        }
    }
}

impl From<String> for Messages {
    fn from(event: String) -> Self {
        match event.as_str() {
            "read" => Messages::Read,
            "post" => Messages::Post,
            "update" => Messages::Update,
            "repost" => Messages::RePost,
            "like" => Messages::Like,
            "unlike" => Messages::UnLike,
            "more" => Messages::FetchMore,
            "" => Messages::Refresh,
            _ => Messages::Unknown(event.to_string()),
        }
    }
}
