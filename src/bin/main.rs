use clap::Parser;
use env_logger;
use log::info;
use rbsky::runner::Runner;
use std::fmt::Debug;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value = "https://bsky.social")]
    pds_host: String,

    /// Debug print
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[command(subcommand)]
    // command: Command,
    command: rbsky::commands::Command,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    Ok(Runner::new(args.pds_host, args.debug)
        .await?
        .run(args.command)
        .await?)
}
