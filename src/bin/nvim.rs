use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use futures::lock::Mutex;
use log::{error, info, trace};
use rbsky::commands::{GetTimelineArgs, LoginArgs};
use rbsky::runner::Runner;
use rbsky::{nvim::EventHandler, surreal::SurrealDB};
use simple_log::LogConfigBuilder;
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

async fn refresh_timeline() -> Result<(), anyhow::Error> {
    Ok(())
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
    let db_reader = Arc::new(Mutex::new(db));
    let db_writer = db_reader.clone();
    let args = Args::parse();

    let task_interval = Duration::from_secs(300);
    let runner = Runner::new(args.pds_host, args.debug).await?;
    runner
        ._login(LoginArgs {
            from_env: true,
            identifier: None,
            password: None,
        })
        .await?;
    tokio::spawn(async move {
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
                    cursor: cursor,
                    limit: 10,
                })
                .await;
            match timeline {
                Ok(data) => {
                    trace!("read timeline {:?}", data);
                    let write_res = db_lock.store_timeline(data, String::from("default")).await;
                    match write_res {
                        Ok(res) => info!("data written successfully: {:?}", res),
                        Err(e) => error!("error while fetching data {:?}", e),
                    }
                }
                Err(e) => {
                    error!("error while fetching data {:?}", e);
                }
            }
            drop(db_lock);
        }
    });
    let mut event_handler = EventHandler::new(db_reader)?;
    event_handler.recv().await?;
    info!("event_handler, done!");
    Ok(())
}
