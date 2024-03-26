use atrium_api::types::string::AtIdentifier;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub enum Command {
    /// Login (Create an authentication session).
    Login(LoginArgs),
    /// Get a view of an actor's home timeline.
    GetTimeline(GetTimelineArgs),
    /// Get a view of an actor's feed.
    GetAuthorFeed(GetAuthorFeedArgs),
    /// Get a list of likes for a given post.
    GetLikes(GetCidUriArgs),
    /// Get a list of reposts for a given post.
    GetRepostedBy(GetCidUriArgs),
    /// Get a list of feeds created by an actor.
    GetActorFeeds(ActorArgs),
    /// Get a view of a hydrated feed.
    GetFeed(UriArgs),
    /// Get a view of a specified list,
    GetPost(UriArgs),
    /// Get a view of a specified list,
    GetListFeed(UriArgs),
    /// Get a list of who an actor follows.
    GetFollows(ActorArgs),
    /// Get a list of an actor's followers.
    GetFollowers(ActorArgs),
    /// Get a list of the list created by an actor.
    GetLists(ActorArgs),
    /// Get detailed info of a specified list.
    GetList(UriArgs),
    /// Get detailed profile view of an actor.
    GetProfile(ActorArgs),
    /// Get a list of notifications.
    ListNotifications(ListNotificationsArgs),
    /// Create a new post.
    CreatePost(CreatePostArgs),
    /// Delete a post.
    DeletePost(UriArgs),
}

#[derive(Parser, Debug)]
pub struct LoginArgs {
    /// Use environment variables BSKYUSERNAME and BSKYPASSWORD for login credentials
    #[arg(long, default_value_t = false)]
    pub from_env: bool,
    /// Handle or other identifier supported by the server for the authenticating user.
    #[arg(short, long)]
    pub identifier: Option<String>,
    /// Password
    #[arg(short, long)]
    pub password: Option<String>,
}

#[derive(Parser, Debug)]
pub struct GetTimelineArgs {
    #[arg(long, default_value_t = String::from("reverse-chronological"))]
    pub algorithm: String,
    #[arg(long)]
    pub cursor: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub limit: u8,
}

#[derive(Parser, Debug)]
pub struct GetAuthorFeedArgs {
    /// Actor's handle or did
    #[arg(short, long, value_parser)]
    pub(crate) actor: Option<AtIdentifier>,
    #[arg(long)]
    pub(crate) cursor: Option<String>,

    #[arg(long)]
    pub(crate) filter: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub(crate) limit: u8,
}

#[derive(Parser, Debug)]
pub struct GetCidUriArgs {
    /// Actor's handle or did
    /// atrium_api::types::string::Cid
    #[arg(short, long, value_parser)]
    pub(crate) cid: Option<atrium_api::types::string::Cid>,
    #[arg(long)]
    pub(crate) cursor: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub(crate) limit: u8,
    #[arg(short, long, value_parser)]
    pub(crate) uri: AtUri,
}

#[derive(Parser, Debug)]
pub struct ActorArgs {
    #[arg(long)]
    pub cursor: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub limit: u8,
    /// Actor's handle or did
    #[arg(short, long, value_parser)]
    pub actor: Option<AtIdentifier>,
}

#[derive(Parser, Debug)]
pub struct UriArgs {
    #[arg(long)]
    pub(crate) cursor: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub(crate) limit: u8,
    /// Record's URI
    #[arg(short, long, value_parser)]
    pub(crate) uri: AtUri,
}

#[derive(Parser, Debug)]
pub struct UriArgsU16 {
    #[arg(long, default_value_t = 10)]
    pub parent_height: u16,
    #[arg(long, default_value_t = 10)]
    pub depth: u16,
    /// Record's URI
    #[arg(short, long, value_parser)]
    pub uri: AtUri,
}

#[derive(Parser, Debug)]
pub struct ListNotificationsArgs {
    #[arg(long)]
    pub(crate) cursor: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub(crate) limit: u8,
    /// Record's URI
    // TODO: CHECK the seen_at since it is a string format datetime
    #[arg(short, long, value_parser)]
    pub(crate) seen_at: atrium_api::types::string::Datetime,
}

#[derive(Parser, Debug)]
pub struct CreatePostArgs {
    /// Post text
    #[arg(short, long)]
    pub(crate) text: String,
    /// Images to embed
    #[arg(short, long)]
    pub(crate) images: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AtUri {
    pub(crate) did: String,
    pub(crate) collection: String,
    pub(crate) rkey: String,
}

impl FromStr for AtUri {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s
            .strip_prefix("at://did:plc:")
            .ok_or(r#"record uri must start with "at://did:plc:""#)?
            .splitn(3, '/')
            .collect::<Vec<_>>();
        Ok(Self {
            did: format!("did:plc:{}", parts[0]),
            collection: parts[1].to_string(),
            rkey: parts[2].to_string(),
        })
    }
}

impl std::fmt::Display for AtUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at://{}/{}/{}", self.did, self.collection, self.rkey)
    }
}
