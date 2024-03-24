use std::sync::Arc;

use atrium_api::app::bsky::feed::defs::FeedViewPost;
use futures::lock::Mutex;
use log::{error, info, trace};
use neovim_lib::{Neovim, RequestHandler, Session};

use crate::surreal::SurrealDB;

enum Messages {
    Read,
    Post,
    RePost,
    Like,
    UnLike,
    Unknown(String),
}

pub struct BskyRequestHandler {
    pub feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPost>>>>,
}

impl RequestHandler for BskyRequestHandler {
    fn handle_request(
        &mut self,
        name: &str,
        args: Vec<neovim_lib::Value>,
    ) -> Result<neovim_lib::Value, neovim_lib::Value> {
        trace!("Received name: {:?}, args: {:?}", name, args);
        match Messages::from(name) {
            Messages::Read => {
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
                                        return Ok(neovim_lib::Value::from(s.as_str()));
                                    }
                                    Err(_) => {
                                        drop(l);
                                        error!("No Data to return: returning nil");
                                        return Ok(neovim_lib::Value::from("nil"));
                                    }
                                }
                            }
                            None => {
                                drop(l);
                                error!("Lock acquired: No data in opt");
                                return Ok(neovim_lib::Value::from("nil"));
                            }
                        }
                    }
                    Err(_) => {
                        error!("Unable to acquire the lock: returning nil");
                        return Ok(neovim_lib::Value::from("the feed is empty"));
                    }
                }
            }
            Messages::Post => {
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
            Messages::Unknown(event) => {
                error!("Uninmplemented");
                return Ok(neovim_lib::Value::from("Unimplemented"));
            }
        }
    }
}

impl BskyRequestHandler {
    pub async fn update_feed(&mut self, db: &Arc<Mutex<SurrealDB>>) -> Result<(), anyhow::Error> {
        trace!("updating read handler feed");
        let db_lock = db.lock().await;
        let cached_feed: Vec<FeedViewPost> = db_lock.read_timeline(String::from("default")).await?;
        trace!("reading the data: {:?}", cached_feed);
        let locked = self.feed.lock();
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
}

pub struct EventHandler {
    pub nvim: Neovim,
    pub db: Arc<Mutex<SurrealDB>>,
}

impl EventHandler {
    pub fn new(db: Arc<Mutex<SurrealDB>>) -> Result<EventHandler, anyhow::Error> {
        let session = Session::new_parent()?;
        let nvim = Neovim::new(session);
        let db = db;
        Ok(EventHandler { nvim, db })
    }

    // TODO: add args to the recv function, add timeline, which timeline should I read
    // Add this in the setup of the binary adding clap
    pub async fn recv(
        &mut self,
        bsky_request_handler: BskyRequestHandler,
    ) -> Result<(), anyhow::Error> {
        let receiver = self
            .nvim
            .session
            .start_event_loop_channel_handler(bsky_request_handler);
        for (event, values) in receiver {
            trace!("Received event: {:?}, values: {:?}", event, values);
        }
        Ok(())
    }
}

impl From<&str> for Messages {
    fn from(event: &str) -> Self {
        match event {
            "read" => Messages::Read,
            "post" => Messages::Post,
            "repost" => Messages::RePost,
            "like" => Messages::Like,
            "unlike" => Messages::UnLike,
            _ => Messages::Unknown(event.to_string()),
        }
    }
}
