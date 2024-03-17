use atrium_api::app::bsky::feed::defs::FeedViewPost;
use env_logger;
use log::trace;
use rbsky::{nvim::EventHandler, surreal::SurrealDB};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // TODO:: Add a file logger for now in config dir, rbsky-nvim.log
    // TODO:: Start a process to update the data stored in surrealdb
    env_logger::init();
    let db = SurrealDB::new().await?;
    let mut event_handler = EventHandler::new(db)?;
    event_handler.recv().await?;
    Ok(())
}
