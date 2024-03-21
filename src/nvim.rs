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

pub struct ReadHandler;

impl RequestHandler for ReadHandler {
    fn handle_request(
        &mut self,
        name: &str,
        args: Vec<neovim_lib::Value>,
    ) -> Result<neovim_lib::Value, neovim_lib::Value> {
        trace!("Received name: {:?}, args: {:?}", name, args);
        Ok(neovim_lib::Value::from(""))
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
        let receiver = self
            .nvim
            .session
            .start_event_loop_channel_handler(ReadHandler);
        trace!("{:?} receiver values", receiver);
        for (event, values) in receiver {
            trace!("Received event: {:?}, values: {:?}", event, values);
            /*
            match Messages::from(event) {
                Messages::Read => {
                    let db_lock = self.db.lock().await;
                    let cached_feed: Vec<FeedViewPost> =
                        db_lock.read_timeline(String::from("default")).await?;
                    trace!("Reading the data: {:?}", cached_feed.first());
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
