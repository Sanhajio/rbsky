use atrium_api::app::bsky::feed::defs::FeedViewPost;
use log::trace;
use neovim_lib::{neovim_api::Buffer, Neovim, NeovimApi, NeovimApiAsync, Session};

use crate::surreal::SurrealDB;

enum Messages {
    Read,
    Post,
    RePost,
    Like,
    UnLike,
    Unknown(String),
}

pub struct EventHandler {
    pub nvim: Neovim,
    pub db: SurrealDB,
}

impl EventHandler {
    pub fn new(db: SurrealDB) -> Result<EventHandler, anyhow::Error> {
        let session = Session::new_parent()?;
        let nvim = Neovim::new(session);
        let db = db;
        Ok(EventHandler { nvim, db })
    }

    // TODO: add args to the recv function, add timeline, which timeline should I read
    // Add this in the setup of the binary adding clap
    pub async fn recv(&mut self) -> Result<(), anyhow::Error> {
        let receiver = self.nvim.session.start_event_loop_channel();
        let mut neosky_buffer: Option<Buffer> = None;
        for (event, values) in receiver {
            trace!("Received event: {:?}, values: {:?}", event, values);
            match Messages::from(event) {
                Messages::Read => {
                    let cached_feed: Vec<FeedViewPost> =
                        self.db.read_timeline(String::from("default")).await?;
                    trace!("Reading the data: {:?}", cached_feed);
                    // self.nvim.call_function(, args)
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
