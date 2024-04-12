use atrium_api::app::bsky::feed;
use clap::Parser;
use env_logger;
use log::info;
use rbsky::{commands::Command, runner::Runner};
use std::io::Write;
use std::{fmt::Debug, os::unix::fs::FileExt};

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
            let res: feed::get_timeline::Output = runner._get_timeline(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetAuthorFeed(args) => {
            let res: feed::get_author_feed::Output = runner._get_author_feed(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetLikes(args) => {
            let res = runner._get_likes(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetPosts(args) => {
            let res = runner._get_post(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetRepostedBy(args) => {
            let res = runner._get_reposted_by(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetActorFeeds(args) => {
            let res = runner._get_actor_feed(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetFeed(args) => {
            let res = runner._get_feed(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetListFeed(args) => {
            let res = runner._get_list_feed(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetFollows(args) => {
            let res = runner._get_follows(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetFollowers(args) => {
            let res = runner._get_followers(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetLists(args) => {
            let res = runner._get_lists(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetList(args) => {
            let res = runner._get_list(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetProfile(args) => {
            let res = runner._get_profile(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::GetBlob(args) => {
            let res = runner._get_blob(args).await?;
            let file_path = "/tmp/file.jpeg";
            let mut file = std::fs::File::create(file_path)?;
            file.write_all(&res)?;
            Ok(())
        }
        Command::ListNotifications(args) => {
            let res = runner._list_notifications(args).await?;
            let json = serde_json::to_string_pretty(&res);
            if let Ok(d) = json {
                println!("{}", d);
            }
            Ok(())
        }
        Command::CreatePost(args) => runner._create_post(args).await,
        Command::DeletePost(args) => runner._delete_post(args).await,
    }
}
