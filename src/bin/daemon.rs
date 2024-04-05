use clap::Parser;
use rbsky::{commands::GetTimelineArgs, runner::Runner, surreal::SurrealDB};
use simple_log::LogConfigBuilder;
use std::fmt::Debug;

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
    let config = LogConfigBuilder::builder()
        .path(String::from("output/daemon.log"))
        .level("trace")
        .size(1 * 100)
        .roll_count(10)
        .output_file()
        .build();
    let _ = simple_log::new(config);
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
    let timeline = db.read_timeline(String::from("default"), None).await?;
    println!("{}", serde_json::to_string_pretty(&timeline)?);
    Ok(())
}
