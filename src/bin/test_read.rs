use log::trace;
use rbsky::surreal::SurrealDB;
use simple_log::LogConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let db = SurrealDB::new().await?;
    let config = LogConfigBuilder::builder()
        .path(String::from("output/test_read.log"))
        .level("trace")
        .size(1 * 100)
        .roll_count(10)
        .output_file()
        .build();
    let _ = simple_log::new(config);

    let cached_feed = db.read_timeline(String::from("default")).await?;
    trace!("Reading the data: {:?}", cached_feed);
    Ok(())
}
