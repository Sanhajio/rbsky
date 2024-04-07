use std::collections::HashMap;
use std::sync::Arc;

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
                    self.update_timeline(None).await?;
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
                    error!("Uninmplemented");
                }
                Messages::Unknown(event) => {
                    error!("Uninmplemented {}", event);
                }
            }
        }
        Ok(())
    }

    // TODO: I'll put all the logic here and refactor afterward
    pub async fn fetch_more(
        &mut self,
        cid: String,
        feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
    ) -> Result<(), anyhow::Error> {
        trace!("fetch more data into the feed handler");
        let db_lock = self.db.lock().await;
        let db = &db_lock.db;
        let _ = db.use_ns("bsky").use_db("timeline").await;
        let query_select_created_at_post = format!(
            r#"SELECT post.record.createdAt as createdAt FROM feed WHERE post.cid='{cid}' LIMIT 1;"#
        );

        let mut result = db.query(query_select_created_at_post).await?;
        let value: Option<HashMap<String, String>> = result.take(0)?;
        trace!("reading post createdAt query data: {:?}", value);
        if let Some(first_result) = value {
            if let Some(created_at) = first_result.get("createdAt") {
                let query_count_posts = format!(
                    r#"SELECT COUNT() as c FROM feed WHERE post.record.createdAt <= '{created_at}' GROUP ALL"#,
                );
                let mut result_number: i32 = 0;
                while result_number < 11 {
                    let mut count_result = db.query(&query_count_posts).await?;
                    let mut cursor: String = created_at.to_string();
                    let count: Option<HashMap<String, i32>> = count_result.take(0)?;
                    trace!("reading the data: {:?}", count);
                    if let Some(count) = count {
                        if let Some(c) = count.get("c") {
                            result_number = *c;
                            if *c > 11 {
                                let more: Vec<FeedViewPostFlat> = db_lock
                                    .read_timeline(
                                        String::from("default"),
                                        Some(format!("createdAt < '{cursor}'")),
                                    )
                                    .await?;
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
                            } else {
                                let new_cursor_time =
                                    chrono::DateTime::parse_from_rfc3339(&cursor)?
                                        .checked_sub_signed(
                                            chrono::Duration::try_minutes(10)
                                                .expect("Unable to convert to minutes"),
                                        )
                                        .expect("Time calculation error")
                                        .to_rfc3339();
                                cursor = String::from(new_cursor_time);
                                let timeline = self
                                    .runner
                                    ._get_timeline(GetTimelineArgs {
                                        algorithm: String::from("reverse-chronological"),
                                        cursor: Some(cursor),
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
        match cursor {
            Some(ref _c) => {}
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
