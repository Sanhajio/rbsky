use atrium_api::app::bsky::feed::defs::FeedViewPost;
use clap::Parser;
use env_logger;
use log::trace;
use rbsky::{commands::GetTimelineArgs, runner::Runner, surreal::SurrealDB};
use std::fmt::Debug;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct NvimDaemonArgs {
    #[arg(short, long, default_value = "https://bsky.social")]
    pds_host: String,

    /// Debug print
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value = "https://bsky.social")]
    pds_host: String,

    /// Debug print
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let args = Args::parse();
    let runner = Runner::new(args.pds_host, args.debug).await?;
    let db = SurrealDB::new().await?;
    let timeline: atrium_api::app::bsky::feed::get_timeline::Output = runner
        ._get_timeline(GetTimelineArgs {
            algorithm: String::from("reverse-chronological"),
            cursor: None,
            limit: 10,
        })
        .await?;
    db.store_timeline(timeline, String::from("default")).await?;
    let cached_feed: Vec<FeedViewPost> = db.read_timeline(String::from("default")).await?;
    trace!("Reading the data: {:?}", cached_feed);
    Ok(())
}
