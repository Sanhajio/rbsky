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

    pub async fn recv(&mut self) -> Result<(), anyhow::Error> {
        let receiver = self.nvim.session.start_event_loop_channel();
        let mut neosky_buffer: Option<Buffer> = None;
        for (event, values) in receiver {
            trace!("Received event: {:?}, values: {:?}", event, values);
            match Messages::from(event) {
                Messages::Read => {
                    // TODO:: I might not need to open a buffer to enter the data there
                    // I would rather take the data into a table and display it from neovim
                    let cached_feed: Vec<FeedViewPost> =
                        self.db.read_timeline(String::from("default")).await?;
                    trace!("Reading the data: {:?}", cached_feed);
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
                    if let Some(ref buffer) = neosky_buffer {
                        self.nvim.set_current_buf(&buffer)?;

                        // TODO:: send a table data to feed
                        let feed_lines: Vec<String> = cached_feed
                            .iter()
                            .map(|post| serde_json::to_string(post).unwrap())
                            .collect();

                        // TODO::
                        buffer.set_lines(&mut self.nvim, 0, -1, true, feed_lines)?;
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
