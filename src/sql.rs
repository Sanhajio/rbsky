use std::collections::HashMap;

use atrium_api::app::bsky::actor::get_profile::Error;
use surrealdb::{engine::local::Db, Surreal};

use crate::surreal::SurrealDB;

pub enum SqlQuery {
    SelectCreatedAt { cid: String },
    CountPostsOlderThan { created_at: String },
    CountPostsNewerThan { created_at: String },
}

impl SqlQuery {
    pub fn to_sql(&self) -> String {
        match self {
            SqlQuery::SelectCreatedAt { cid } => format!(
                r#"SELECT post.record.createdAt as createdAt FROM feed WHERE post.cid={} LIMIT 1;"#,
                cid
            ),
            SqlQuery::CountPostsOlderThan { created_at } => format!(
                r#"SELECT COUNT() as c FROM feed WHERE post.record.createdAt <= '{}' GROUP ALL"#,
                created_at
            ),
            SqlQuery::CountPostsNewerThan { created_at } => format!(
                r#"SELECT COUNT() as c FROM feed WHERE post.record.createdAt >= '{}' GROUP ALL"#,
                created_at
            ),
        }
    }
}

pub struct Querier {
    db: Surreal<Db>,
}

impl Querier {
    pub fn new(db: Surreal<Db>) -> Self {
        Querier { db }
    }

    pub async fn run_query(&self, query: &str) -> Result<surrealdb::Response, anyhow::Error> {
        let result: surrealdb::Response = self.db.query(query).await?;
        Ok(result)
    }

    pub async fn count_posts_older_than(&self, created_at: &str) -> Result<i32, anyhow::Error> {
        let query = SqlQuery::CountPostsOlderThan {
            created_at: created_at.to_string(),
        };
        let sql = query.to_sql();
        let mut result: surrealdb::Response = self.run_query(&sql).await?;
        let count_map: Option<HashMap<String, i32>> = result.take(0)?;
        if let Some(count_map) = count_map {
            if let Some(count) = count_map.get("c") {
                return Ok(*count);
            }
        }
        Err(anyhow::Error::msg(format!("Failed to get count for {sql}")))
    }

    pub async fn count_posts_newer_than(&self, created_at: &str) -> Result<i32, anyhow::Error> {
        let query = SqlQuery::CountPostsNewerThan {
            created_at: created_at.to_string(),
        };
        let sql = query.to_sql();
        let mut result = self.run_query(&sql).await?;
        let count_map: Option<HashMap<String, i32>> = result.take(0)?;
        if let Some(count_map) = count_map {
            if let Some(count) = count_map.get("c") {
                return Ok(*count);
            }
        }
        Err(anyhow::Error::msg(format!("Failed to get count for {sql}")))
    }

    pub async fn select_created_at(&self, cid: &str) -> Result<String, anyhow::Error> {
        let query = SqlQuery::SelectCreatedAt {
            cid: cid.to_string(),
        };
        let sql = query.to_sql();
        let mut result = self.run_query(&sql).await?;
        let result_map: Option<HashMap<String, String>> = result.take(0)?;
        if let Some(record) = result_map {
            if let Some(created_at) = record.get("createdAt") {
                return Ok(created_at.to_string());
            }
        }
        Err(anyhow::Error::msg(format!("Failed to get count for {sql}")))
    }
}
