use crate::commands::{
    ActorArgs, Command, CreatePostArgs, GetAuthorFeedArgs, GetCidUriArgs, GetTimelineArgs,
    ListNotificationsArgs, LoginArgs, UriArgs,
};
use crate::store::SimpleJsonFileSessionStore;
use anyhow::{Context, Result};
use atrium_api::agent::{store::SessionStore, AtpAgent};
use atrium_api::app::bsky::actor;
use atrium_api::app::bsky::feed;
use atrium_api::app::bsky::graph;
use atrium_api::app::bsky::notification;
use atrium_api::types::string::{AtIdentifier, Datetime, Handle};
use atrium_api::types::LimitedNonZeroU8;
use atrium_xrpc_client::reqwest::ReqwestClient;
use log::{error, info};
use std::ffi::OsStr;
use std::path::PathBuf;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncReadExt;

pub struct Runner {
    agent: AtpAgent<SimpleJsonFileSessionStore, ReqwestClient>,
    debug: bool,
    session_path: PathBuf,
    config_dir: PathBuf,
    handle: Option<Handle>,
}

// TODO: split Runner and run
impl Runner {
    pub async fn new(pds_host: String, debug: bool) -> Result<Self> {
        let config_dir = dirs::config_dir()
            .with_context(|| format!("No config dir: {:?}", dirs::config_dir()))?;
        let dir = config_dir.join("bsky");
        create_dir_all(&dir).await?;
        let session_path = dir.join("session.json");
        let store = SimpleJsonFileSessionStore::new(session_path.clone());
        let session = store.get_session().await;
        let handle = session.as_ref().map(|s| s.handle.clone());
        let agent = AtpAgent::new(ReqwestClient::new(pds_host), store);
        if let Some(s) = &session {
            agent.resume_session(s.clone()).await?;
        }
        Ok(Self {
            agent,
            debug,
            config_dir,
            session_path,
            handle,
        })
    }

    // TODO: Check if this reads the stored session
    pub async fn _login(&self, args: LoginArgs) -> Result<()> {
        match (args.from_env, args.identifier, args.password) {
            (true, _, _) => {
                let identifier = std::env::var("BSKYUSERNAME")
                    .expect("Environment variable BSKYUSERNAME not set");
                let password = std::env::var("BSKYPASSWORD")
                    .expect("Environment variable BSKYPASSWORD not set");
                info!("Login with BSKYPASSWORD and BSKYPASSWORD, login successful! Saved session to {:?}", self.session_path);
                self.agent.login(identifier, password).await?;
            }
            (_, Some(identifier), Some(password)) => {
                self.agent.login(identifier, password).await?;
                info!(
                    "Login with indentifier and password, Login successful! Saved session to {:?}",
                    self.session_path
                );
            }
            _ => {
                error!("Invalid login arguments: Must specify from_env or both identifier and password");
            }
        }
        Ok(())
    }

    pub async fn _get_author_feed(
        &self,
        args: GetAuthorFeedArgs,
    ) -> Result<feed::get_author_feed::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_author_feed(atrium_api::app::bsky::feed::get_author_feed::Parameters {
                actor: args
                    .actor
                    .or(self.handle.clone().map(AtIdentifier::Handle))
                    .with_context(|| "Not logged in")?,
                cursor: args.cursor,
                filter: args.filter,
                limit: Some(limit),
            })
            .await?)
    }

    pub async fn _get_timeline(
        &self,
        args: GetTimelineArgs,
    ) -> Result<feed::get_timeline::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_timeline(atrium_api::app::bsky::feed::get_timeline::Parameters {
                algorithm: Some(args.algorithm),
                cursor: args.cursor,
                limit: Some(limit),
            })
            .await?)
    }

    pub async fn _get_likes(
        &self,
        args: GetCidUriArgs,
    ) -> Result<feed::get_likes::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_likes(atrium_api::app::bsky::feed::get_likes::Parameters {
                cid: args.cid,
                cursor: args.cursor,
                limit: Some(limit),
                uri: args.uri.to_string(),
            })
            .await?)
    }

    pub async fn _get_reposted_by(
        &self,
        args: GetCidUriArgs,
    ) -> Result<feed::get_reposted_by::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_reposted_by(atrium_api::app::bsky::feed::get_reposted_by::Parameters {
                cid: args.cid,
                cursor: args.cursor,
                limit: Some(limit),
                uri: args.uri.to_string(),
            })
            .await?)
    }

    pub async fn _get_actor_feed(
        &self,
        args: ActorArgs,
    ) -> Result<feed::get_actor_feeds::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_actor_feeds(atrium_api::app::bsky::feed::get_actor_feeds::Parameters {
                actor: args
                    .actor
                    .or(self.handle.clone().map(AtIdentifier::Handle))
                    .with_context(|| "Not logged in")?,
                cursor: None,
                limit: Some(limit),
            })
            .await?)
    }

    pub async fn _get_list_feed(
        &self,
        args: UriArgs,
    ) -> Result<feed::get_list_feed::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_list_feed(atrium_api::app::bsky::feed::get_list_feed::Parameters {
                cursor: args.cursor,
                limit: Some(limit),
                list: args.uri.to_string(),
            })
            .await?)
    }

    pub async fn _get_feed(&self, args: UriArgs) -> Result<feed::get_feed::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .feed
            .get_feed(atrium_api::app::bsky::feed::get_feed::Parameters {
                cursor: args.cursor,
                limit: Some(limit),
                feed: args.uri.to_string(),
            })
            .await?)
    }

    pub async fn _get_follows(
        &self,
        args: ActorArgs,
    ) -> Result<graph::get_follows::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .graph
            .get_follows(atrium_api::app::bsky::graph::get_follows::Parameters {
                actor: args
                    .actor
                    .or(self.handle.clone().map(AtIdentifier::Handle))
                    .with_context(|| "Not logged in")?,
                cursor: None,
                limit: Some(limit),
            })
            .await?)
    }

    pub async fn _get_followers(
        &self,
        args: ActorArgs,
    ) -> Result<graph::get_followers::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .graph
            .get_followers(atrium_api::app::bsky::graph::get_followers::Parameters {
                actor: args
                    .actor
                    .or(self.handle.clone().map(AtIdentifier::Handle))
                    .with_context(|| "Not logged in")?,
                cursor: None,
                limit: Some(limit),
            })
            .await?)
    }

    pub async fn _get_lists(
        &self,
        args: ActorArgs,
    ) -> Result<graph::get_lists::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .graph
            .get_lists(atrium_api::app::bsky::graph::get_lists::Parameters {
                actor: args
                    .actor
                    .or(self.handle.clone().map(AtIdentifier::Handle))
                    .with_context(|| "Not logged in")?,
                cursor: None,
                limit: Some(limit),
            })
            .await?)
    }

    pub async fn _get_list(&self, args: UriArgs) -> Result<graph::get_list::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .graph
            .get_list(atrium_api::app::bsky::graph::get_list::Parameters {
                cursor: args.cursor,
                limit: Some(limit),
                list: args.uri.to_string(),
            })
            .await?)
    }

    pub async fn _get_profile(
        &self,
        args: ActorArgs,
    ) -> Result<actor::get_profile::Output, anyhow::Error> {
        Ok(self
            .agent
            .api
            .app
            .bsky
            .actor
            .get_profile(atrium_api::app::bsky::actor::get_profile::Parameters {
                actor: args
                    .actor
                    .or(self.handle.clone().map(AtIdentifier::Handle))
                    .with_context(|| "Not logged in")?,
            })
            .await?)
    }

    pub async fn _list_notifications(
        &self,
        args: ListNotificationsArgs,
    ) -> Result<notification::list_notifications::Output, anyhow::Error> {
        let limit: LimitedNonZeroU8<100> = args.limit.try_into().expect("within limit");
        Ok(self
            .agent
            .api
            .app
            .bsky
            .notification
            .list_notifications(
                atrium_api::app::bsky::notification::list_notifications::Parameters {
                    cursor: args.cursor,
                    limit: Some(limit),
                    seen_at: Some(args.seen_at),
                },
            )
            .await?)
    }

    // TODO: Reword this function to make create post args more flexible
    pub async fn _create_post(&self, args: CreatePostArgs) -> Result<(), anyhow::Error> {
        let mut images = Vec::new();
        for image in &args.images {
            if let Ok(mut file) = File::open(image).await {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).await.expect("read image file");
                let output = self
                    .agent
                    .api
                    .com
                    .atproto
                    .repo
                    .upload_blob(buf)
                    .await
                    .expect("upload blob");
                images.push(atrium_api::app::bsky::embed::images::Image {
                    alt: image
                        .file_name()
                        .map(OsStr::to_string_lossy)
                        .unwrap_or_default()
                        .into(),
                    aspect_ratio: None,
                    image: output.blob,
                })
            }
        }
        let embed = Some(
            atrium_api::app::bsky::feed::post::RecordEmbedEnum::AppBskyEmbedImagesMain(Box::new(
                atrium_api::app::bsky::embed::images::Main { images },
            )),
        );
        let res = &self
            .agent
            .api
            .com
            .atproto
            .repo
            .create_record(atrium_api::com::atproto::repo::create_record::Input {
                collection: "app.bsky.feed.post".parse().expect("valid"),
                record: atrium_api::records::Record::AppBskyFeedPost(Box::new(
                    atrium_api::app::bsky::feed::post::Record {
                        created_at: Datetime::now(),
                        embed,
                        entities: None,
                        facets: None,
                        labels: None,
                        langs: None,
                        reply: None,
                        tags: None,
                        text: args.text,
                    },
                )),
                repo: self.handle.clone().with_context(|| "Not logged in")?.into(),
                rkey: None,
                swap_commit: None,
                validate: None,
            })
            .await?;
        info!("post executed succesffully returning {:?}", res);
        Ok(())
    }

    pub async fn _delete_post(&self, args: UriArgs) -> Result<(), anyhow::Error> {
        let res = self
            .agent
            .api
            .com
            .atproto
            .repo
            .delete_record(atrium_api::com::atproto::repo::delete_record::Input {
                collection: "app.bsky.feed.post".parse().expect("valid"),
                repo: self.handle.clone().with_context(|| "Not logged in")?.into(),
                rkey: args.uri.rkey.clone(),
                swap_commit: None,
                swap_record: None,
            })
            .await?;
        let rkey = args.uri.rkey;
        info!("Successfully deleted post: {:?}, res: {:?}", rkey, res);
        Ok(())
    }
}
