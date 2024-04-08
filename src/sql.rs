use std::collections::HashMap;
use surrealdb::{engine::local::Db, Surreal};

pub enum SqlQuery {
    SelectCreatedAt {
        cid: String,
    },
    CountPostsOlderThan {
        created_at: String,
    },
    CountRecentPostsOlderThan {
        created_at: String,
        lower_limit: String,
    },
    CountPostsNewerThan {
        created_at: String,
    },
    GetPost {
        cid: String,
    },
    ReadTimeline {
        filter: Option<String>,
    },
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
            SqlQuery::CountRecentPostsOlderThan {
                created_at,
                lower_limit,
            } => format!(
                r#"SELECT COUNT() as c FROM feed WHERE post.record.createdAt <= '{}' and post.record.createdAt > '{}' GROUP ALL"#,
                created_at, lower_limit
            ),
            SqlQuery::CountPostsNewerThan { created_at } => format!(
                r#"SELECT COUNT() as c FROM feed WHERE post.record.createdAt >= '{}' GROUP ALL"#,
                created_at
            ),
            SqlQuery::GetPost { cid } => format!(
                r#"SELECT post[*], post.record.createdAt as createdAt, reply.parent as parent, reply.root as root, reason OMIT post.id, parent.id, root.id FROM feed WHERE post.cid == {} FETCH post.author, parent, root, parent.author, root.author;"#,
                cid
            ),
            SqlQuery::ReadTimeline { filter } => {
                let base_query = "SELECT post[*], post.record.createdAt as createdAt, reply.parent as parent, reply.root as root, reason OMIT post.id, parent.id, root.id FROM feed";
                let mut query = base_query.to_string();
                if let Some(f) = filter {
                    query = format!("{} WHERE {}", query, f);
                }
                query.push_str(
                    " ORDER BY createdAt DESC LIMIT 10 FETCH post.author, parent, root, parent.author, root.author;",
                );
                query
            }
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
        Ok(0)
        // Err(anyhow::Error::msg(format!("Failed to get count for {sql}")))
    }

    pub async fn count_recent_posts_older_than(
        &self,
        created_at: &str,
        lower_limit: &str,
    ) -> Result<i32, anyhow::Error> {
        let query = SqlQuery::CountRecentPostsOlderThan {
            created_at: created_at.to_string(),
            lower_limit: lower_limit.to_string(),
        };
        let sql = query.to_sql();
        let mut result: surrealdb::Response = self.run_query(&sql).await?;
        let count_map: Option<HashMap<String, i32>> = result.take(0)?;
        if let Some(count_map) = count_map {
            if let Some(count) = count_map.get("c") {
                return Ok(*count);
            }
        }
        Ok(0)
        // Err(anyhow::Error::msg(format!("Failed to get count for {sql}")))
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
        Ok(0)
        // Err(anyhow::Error::msg(format!("Failed to get count for {sql}")))
    }

    pub async fn read_timeline(
        &self,
        filter: Option<String>,
    ) -> Result<Vec<crate::nvim::FeedViewPostFlat>, anyhow::Error> {
        let query = SqlQuery::ReadTimeline { filter };
        let sql = query.to_sql();
        let mut result = self.db.query(&sql).await?;
        let value: Vec<crate::nvim::FeedViewPostFlat> = result.take(0)?;
        Ok(value)
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
        Err(anyhow::Error::msg(format!("Failed to get {sql}")))
    }

    pub async fn get_post(
        &self,
        cid: String,
    ) -> Result<Option<crate::nvim::FeedViewPostFlat>, anyhow::Error> {
        let query = SqlQuery::GetPost {
            cid: cid.to_string(),
        };
        let sql = query.to_sql();
        let mut result = self.run_query(&sql).await?;
        let value: Vec<crate::nvim::FeedViewPostFlat> = result.take(0)?;
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value[0].clone()))
        }
    }
}
