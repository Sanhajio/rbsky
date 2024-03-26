use atrium_api::{
    app::bsky::{
        actor::defs::{ProfileViewBasic, ProfileViewDetailed},
        feed::defs::{FeedViewPost, PostView, ReplyRefRootEnum},
    },
    types::string::{AtIdentifier, Did},
};
use clap::Parser;
use env_logger;
use log::trace;
use rbsky::{
    commands::{ActorArgs, AtUri, GetTimelineArgs, UriArgsU16},
    runner::Runner,
    surreal::SurrealDB,
};
use std::{fmt::Debug, str::FromStr};

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
    for f in timeline.feed {
        println!("------");
        println!("reply");
        let reply = f.reply;
        match reply {
            Some(r) => {
                let root = r.root;
                match root {
                    ReplyRefRootEnum::PostView(r) => {
                        let v = serde_json::to_string_pretty(&r).unwrap();
                        println!("{}", v);
                        let uri: AtUri = AtUri::from_str(&r.uri).unwrap();
                        let post_thread = runner
                            ._get_post_thread(UriArgsU16 {
                                parent_height: 100,
                                depth: 100,
                                uri,
                            })
                            .await?;
                        let v = serde_json::to_string_pretty(&post_thread).unwrap();
                        println!("{}", v);
                    }
                    _ => println!("Not a thread post"),
                }
            }
            _ => println!("Not a thread post"),
        };
    }
    /*
    let post: FeedViewPost = timeline.feed[0].clone();
    let author: ProfileViewBasic = post.post.author;
    let json = serde_json::to_string_pretty(&post);
    if let Ok(d) = json {
        println!("{}", d);
    }
    let at_identifier = AtIdentifier::from_str("did:plc:pr36ekeq2a55ujuvdbk6yuds");
    if let Ok(atid) = at_identifier {
        let profile: ProfileViewDetailed = runner
            ._get_profile(ActorArgs {
                actor: Some(atid),
                cursor: None,
                limit: 10,
            })
            .await?;
        db.store_author(profile).await?;
    }
    */
    Ok(())
}
