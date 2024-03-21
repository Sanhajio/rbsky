use std::sync::Arc;

use anyhow::{Context, Result};
use atrium_api::app::bsky::feed::defs::FeedViewPost;
use clap::Parser;
use futures::lock::Mutex;
use log::{error, info, trace};
use rbsky::commands::{GetTimelineArgs, LoginArgs};
use rbsky::nvim::BskyRequestHandler;
use rbsky::runner::Runner;
use rbsky::{nvim::EventHandler, surreal::SurrealDB};
use simple_log::{new, LogConfigBuilder};
use tokio::fs::create_dir_all;
use tokio::time::{self, Duration};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value = "https://bsky.social")]
    pds_host: String,

    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

async fn refresh_timeline(
    db_writer: Arc<Mutex<SurrealDB>>,
    task_interval: Duration,
    runner: Runner,
    nvim_feed: Arc<std::sync::Mutex<Option<Vec<FeedViewPost>>>>,
) -> Result<(), anyhow::Error> {
    let mut interval = time::interval(task_interval);
    loop {
        interval.tick().await;
        trace!("executed background task");
        // : atrium_api::app::bsky::feed::get_timeline::Output
        let db_lock = db_writer.lock().await;
        let cursor_res = db_lock.get_latest_cursor(String::from("default")).await;
        let mut cursor: Option<String> = None;
        match cursor_res {
            Ok(res) => {
                info!("found cursor at: {:?}", res);
                cursor = res;
            }
            Err(e) => error!("error while fetching cursor data {:?}", e),
        }
        let timeline = runner
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
                let nvim_feed_lock = nvim_feed.lock();
                match nvim_feed_lock {
                    Ok(mut l) => {
                        info!("nvim_feed_lock Acquired Lock Updating Data");
                        *l = Some(data.feed.clone());
                        info!("nvim_feed_lock Droping Lock");
                        drop(l);
                    }
                    Err(_) => error!("Unable to aquire the lock"),
                }
            }
            Err(e) => {
                error!("error while fetching data {:?}", e);
            }
        }
        drop(db_lock);
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // TODO:: Add a file logger for now in config dir, rbsky-nvim.log
    // TODO:: Start a process to update the data stored in surrealdb
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
    let db = SurrealDB::new().await?;
    let nvim_feed_reader = Arc::new(std::sync::Mutex::new(None));
    let nvim_feed_writer = nvim_feed_reader.clone();
    let bsky_request_handler = BskyRequestHandler {
        feed: nvim_feed_reader,
    };
    let db_reader = Arc::new(Mutex::new(db));
    let db_writer = db_reader.clone();
    let args = Args::parse();

    let task_interval = Duration::from_secs(30);
    let runner = Runner::new(args.pds_host, args.debug).await?;
    runner
        ._login(LoginArgs {
            from_env: true,
            identifier: None,
            password: None,
        })
        .await?;

    tokio::spawn(async move {
        if let Err(e) = refresh_timeline(db_writer, task_interval, runner, nvim_feed_writer).await {
            error!("Error in refresh_timeline: {:?}", e);
        }
    });
    let mut event_handler = EventHandler::new(db_reader)?;
    event_handler.recv(bsky_request_handler).await?;
    info!("event_handler, done!");
    Ok(())
}
