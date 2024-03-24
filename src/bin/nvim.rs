use std::sync::Arc;

use anyhow::{Context, Result};
use atrium_api::app::bsky::feed::defs::FeedViewPost;
use clap::Parser;
use futures::lock::Mutex;
use log::{error, info};
use rbsky::commands::LoginArgs;
use rbsky::nvim::BskyRequestHandler;
use rbsky::runner::Runner;
use rbsky::{nvim::EventHandler, surreal::SurrealDB};
use simple_log::LogConfigBuilder;
use tokio::fs::create_dir_all;
use tokio::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value = "https://bsky.social")]
    pds_host: String,

    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[arg(short, long, default_value_t = false)]
    auto_update: bool,
}

async fn init() -> Result<(), anyhow::Error> {
    let config_dir =
        dirs::config_dir().with_context(|| format!("No config dir: {:?}", dirs::config_dir()))?;
    let dir = config_dir.join("bsky");
    create_dir_all(&dir).await?;
    let path = dir.join("rbsky.log");
    if let Some(path) = path.to_str() {
        let config = LogConfigBuilder::builder()
            .path(String::from(path))
            .level("trace")
            .size(1 * 100)
            .roll_count(10)
            .output_file()
            .build();
        let _ = simple_log::new(config);
    }
    info!("Logger Initialized");
    Ok(())
}

async fn auto_update(
    db: Arc<Mutex<SurrealDB>>,
    runner: Runner,
    nvim_feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPost>>>>,
) -> Result<(), anyhow::Error> {
    let task_interval = Duration::from_secs(30);
    let mut event_handler_bg = EventHandler::new(db, runner)?;
    tokio::spawn(async move {
        if let Err(e) = event_handler_bg
            .refresh_timeline(task_interval, nvim_feed)
            .await
        {
            error!("Error in refresh_timeline: {:?}", e);
        }
    });
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init().await?;
    let db = SurrealDB::new().await?;
    let nvim_feed_reader = Arc::new(std::sync::Mutex::new(None));
    let nvim_feed_writer = nvim_feed_reader.clone();

    let db_reader = Arc::new(Mutex::new(db));
    let db_writer = db_reader.clone();
    let bsky_request_handler = BskyRequestHandler {
        feed: nvim_feed_reader,
    };
    let args = Args::parse();

    let pds_host = args.pds_host;
    let runner = Runner::new(pds_host.clone(), args.debug).await?;
    let runner_bg = Runner::new(pds_host.clone(), args.debug).await?;
    runner
        ._login(LoginArgs {
            from_env: true,
            identifier: None,
            password: None,
        })
        .await?;

    let mut event_handler = EventHandler::new(db_reader, runner)?;
    if args.auto_update {
        let _ = auto_update(db_writer, runner_bg, nvim_feed_writer).await;
    }
    event_handler.recv(bsky_request_handler).await?;
    info!("event_handler, done!");
    Ok(())
}
