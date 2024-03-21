use std::sync::Arc;

use atrium_api::app::bsky::feed::defs::FeedViewPost;
use futures::lock::Mutex;
use log::{error, trace};
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

pub struct ReadHandler {
    pub feed: Option<Vec<FeedViewPost>>,
}

impl RequestHandler for ReadHandler {
    fn handle_request(
        &mut self,
        name: &str,
        args: Vec<neovim_lib::Value>,
    ) -> Result<neovim_lib::Value, neovim_lib::Value> {
        trace!("Received name: {:?}, args: {:?}", name, args);
        match &self.feed {
            Some(f) => {
                let feed_json = serde_json::to_string(&f[0..2]);
                match feed_json {
                    Ok(str) => {
                        trace!("Read Handler has value: {:?}", str);
                        return Ok(neovim_lib::Value::from(str.as_str()));
                    }
                    Err(_) => Ok(neovim_lib::Value::from("the feed is empty")),
                }
            }
            None => {
                return Ok(neovim_lib::Value::from("the feed is empty"));
            }
        }
    }
}

impl ReadHandler {
    pub async fn update_feed(&mut self, db: &Arc<Mutex<SurrealDB>>) -> Result<(), anyhow::Error> {
        trace!("updating read handler feed");
        let db_lock = db.lock().await;
        let cached_feed: Vec<FeedViewPost> = db_lock.read_timeline(String::from("default")).await?;
        trace!("reading the data: {:?}", cached_feed);
        self.feed = Some(cached_feed);
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
    pub async fn recv(&mut self) -> Result<(), anyhow::Error> {
        let mut read_handler = ReadHandler { feed: None };
        let receiver = self
            .nvim
            .session
            .start_event_loop_channel_handler(read_handler);
        for (event, values) in receiver {
            trace!("Received event: {:?}, values: {:?}", event, values);
            /*
            match Messages::from(event) {
                Messages::Read => {
                    let db_lock = self.db.lock().await;
                    let cached_feed: vec<feedviewpost> =
                        db_lock.read_timeline(string::from("default")).await?;
                    trace!("reading the data: {:?}", cached_feed.first());
                    drop(db_lock);
                    let feed_json = serde_json::to_string(&cached_feed.first())?;
                    println!("{}", feed_json)
                }
                Messages::Post => {
                    // TODO:: Add an nui or any other ui plugin to add a Post like interface
                }
                Messages::RePost => {
                    //
                }
                Messages::Like => {
                    //
                }
                Messages::UnLike => {
                    //
                }
                Messages::Unknown(event) => {
                    //
                }
            }
            */
        }
        Ok(())
    }
}

impl From<String> for Messages {
    fn from(event: String) -> Self {
        match &event[..] {
            "read" => Messages::Read,
            "post" => Messages::Post,
            "repost" => Messages::RePost,
            "like" => Messages::Like,
            "unlike" => Messages::UnLike,
            _ => Messages::Unknown(event),
        }
    }
}
