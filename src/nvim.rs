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

pub struct BskyRequestHandler {
    pub feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPostFlat>>>>,
}

impl RequestHandler for BskyRequestHandler {
    fn handle_request(
        &mut self,
        name: &str,
        args: Vec<neovim_lib::Value>,
    ) -> Result<neovim_lib::Value, neovim_lib::Value> {
        trace!("Received name: {:?}, args: {:?}", name, args);
        match Messages::from(name) {
            Messages::Read => Ok(self.handle_read_request()),
            Messages::FetchMore => {
                // args[0]:
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
        info!("request_handler: acquiring lock");
        let locked = self.feed.lock();
        match locked {
            Ok(l) => {
                let opt = l.clone();
                match opt {
                    Some(f) => {
                        let feed_json = serde_json::to_string(&f);
                        match feed_json {
                            Ok(s) => {
                                info!("request_handler: lock acquired");
                                trace!("Read Handler has value: {:?}", s);
                                drop(l);
                                info!("request_handler: lock dropped");
                                return neovim_lib::Value::from(s.as_str());
                            }
                            Err(_) => {
                                drop(l);
                                error!("No Data to return: returning nil");
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
                return neovim_lib::Value::from("the feed is empty");
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
            trace!("Received event: {:?}, values: {:?}", event, values);
            match Messages::from(event) {
                Messages::Read => {
                    error!("Uninmplemented");
                }
                Messages::Post => {
                    error!("Uninmplemented");
                }
                Messages::Update => {
                    self.update_timeline().await?;
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
                    // TODO: I need to query the db and return the result from the cid from neovim,
                    // if there are no result update the timeline from that createdAt minus 1 hour
                    // return the result
                    // args[0] Last CID
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
        trace!("updating read handler feed");
        let db_lock = self.db.lock().await;
        let db = &db_lock.db;
        let _ = db.use_ns("bsky").use_db("timeline").await;
        let query = format!(
            r#"SELECT post.record.createdAt as createdAt FROM feed WHERE cid={cid} LIMIT 1;"#
        );
        let mut result = db.query(query).await?;
        trace!("reading the data: {:?}", result);
        let value: Option<HashMap<String, String>> = result.take(0)?;
        trace!("reading the data: {:?}", value);
        if let Some(first_result) = value {
            if let Some(created_at) = first_result.get("createdAt") {
                let cursor = created_at;
                let count_query = format!(
                    r#"SELECT COUNT() as c FROM feed WHERE createdAt < '{cursor}' GROUP ALL"#,
                );
                let mut count_result = db.query(&count_query).await?;
                let count: Option<HashMap<String, i32>> = count_result.take(0)?;
                trace!("reading the data: {:?}", count);
                if let Some(count) = count {
                    if let Some(c) = count.get("c") {
                        if *c > 0 {
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
                                        existing_feed.extend(more.clone()); // Merge cached_feed into existing_feed
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
    pub async fn update_timeline(&mut self) -> Result<(), anyhow::Error> {
        let db_lock = self.db.lock().await;
        let cursor_res = db_lock.get_latest_cursor(String::from("default")).await;
        let mut cursor: Option<String> = None;
        match cursor_res {
            Ok(res) => {
                info!("found cursor at: {:?}", res);
                cursor = res;
            }
            Err(e) => error!("error while fetching cursor data {:?}", e),
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
            self.update_timeline().await?;
            let db_lock = self.db.lock().await;
            let data: Vec<FeedViewPostFlat> =
                db_lock.read_timeline(String::from("default"), None).await?;
            let nvim_feed_lock = nvim_feed.lock();
            match nvim_feed_lock {
                Ok(mut l) => {
                    info!("nvim_feed_lock Acquired Lock Updating Data");
                    if let Some(ref mut existing_feed) = *l {
                        existing_feed.extend(data.clone()); // Merge cached_feed into existing_feed
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
