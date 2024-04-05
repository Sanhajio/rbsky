use std::sync::Arc;

use anyhow::{Context, Result};
use futures::lock::Mutex;
use rbsky::nvim::{BskyRequestHandler, FeedViewPostFlat};
use rbsky::runner::Runner;
use rbsky::{nvim::EventHandler, surreal::SurrealDB};
use simple_log::LogConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = LogConfigBuilder::builder()
        .path(String::from("output/test_read.log"))
        .level("trace")
        .size(1 * 100)
        .roll_count(10)
        .output_file()
        .build();
    let _ = simple_log::new(config);
    let db = SurrealDB::new().await?;
    let nvim_feed_reader = Arc::new(std::sync::Mutex::new(None));
    let bsky_request_handler = BskyRequestHandler {
        feed: nvim_feed_reader,
    };
    let runner = Runner::new(String::from("https://bsky.social"), false).await?;

    let db_reader = Arc::new(Mutex::new(db));
    let mut event_handler = EventHandler::new(db_reader, runner)?;
    event_handler
        .fetch_more(String::from(
            "bafyreif3idvj3shzxlzkybsazlar7rhfros42pnndkcfknis2je7sq6yx4",
        ))
        .await?;
    Ok(())
}
