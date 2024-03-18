use std::sync::Arc;

use atrium_api::app::bsky::feed::defs::FeedViewPost;
use futures::lock::Mutex;
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
        let receiver = self.nvim.session.start_event_loop_channel();
        let mut neosky_buffer: Option<Buffer> = None;
        let buffers: Vec<Buffer> = self.nvim.list_bufs()?;
        for buf in buffers {
            let name = buf.get_name(&mut self.nvim)?;
            if name.ends_with("neosky.social") {
                neosky_buffer = Some(buf);
                break;
            }
        }
        if neosky_buffer.is_none() {
            self.nvim.command("enew")?;
            self.nvim.command("file neosky.social")?;
            neosky_buffer = Some(self.nvim.get_current_buf()?)
        }
        for (event, values) in receiver {
            trace!("Received event: {:?}, values: {:?}", event, values);
            match Messages::from(event) {
                Messages::Read => {
                    let db_lock = self.db.lock().await;
                    let cached_feed: Vec<FeedViewPost> =
                        db_lock.read_timeline(String::from("default")).await?;
                    trace!("Reading the data: {:?}", cached_feed);
                    drop(db_lock);
                    let feed_json = serde_json::to_string(&cached_feed)?;

                    if let Some(ref buffer) = neosky_buffer {
                        self.nvim.set_current_buf(&buffer)?;
                        // TODO: Check if this is a global or a local variable
                        self.nvim.set_var(
                            r#"neosky_feed"#,
                            neovim_lib::Value::String(feed_json.into()),
                        )?;
                    }
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
