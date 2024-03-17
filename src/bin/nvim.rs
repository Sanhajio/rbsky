use anyhow::{Context, Result};
use log::{info, trace};
use rbsky::{nvim::EventHandler, surreal::SurrealDB};
use simple_log::LogConfigBuilder;
use tokio::fs::create_dir_all;

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
    let mut event_handler = EventHandler::new(db)?;
    event_handler.recv().await?;
    info!("event_handler, done!");
    Ok(())
}
