use clap::Parser;
use env_logger;
use log::info;
use rbsky::{commands::Command, runner::Runner};
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
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let args = Args::parse();
    let runner = Runner::new(args.pds_host, args.debug).await?;

    let command = args.command;

    match command {
        Command::Login(args) => runner._login(args).await,
        Command::GetTimeline(args) => {
            let res = runner._get_timeline(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetAuthorFeed(args) => {
            let res = runner._get_author_feed(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetLikes(args) => {
            let res = runner._get_likes(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetRepostedBy(args) => {
            let res = runner._get_reposted_by(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetActorFeeds(args) => {
            let res = runner._get_actor_feed(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetFeed(args) => {
            let res = runner._get_feed(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetListFeed(args) => {
            let res = runner._get_list_feed(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetFollows(args) => {
            let res = runner._get_follows(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetFollowers(args) => {
            let res = runner._get_followers(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetLists(args) => {
            let res = runner._get_lists(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetList(args) => {
            let res = runner._get_list(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::GetProfile(args) => {
            let res = runner._get_profile(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::ListNotifications(args) => {
            let res = runner._list_notifications(args).await;
            info!("{:?}", res);
            Ok(())
        }
        Command::CreatePost(args) => runner._create_post(args).await,
        Command::DeletePost(args) => runner._delete_post(args).await,
    }
}
