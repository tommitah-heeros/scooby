use chrono::{DateTime, Utc};
use colored::Colorize;
use colored_json::to_colored_json_auto;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str};
use std::{error::Error, fmt::Display, path::Path, time::SystemTime};
use tokio::fs;
use turso::{Builder, Connection, Row};

use crate::http::ResponseParts;

pub struct DbStoreArgs {
    pub method: String,
    pub service: String,
    pub url: String,
    pub route_url: String,
    pub payload: Option<serde_json::Value>,
}

pub struct Db {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoobyRequest {
    method: String,
    service: String,
    route_url: String,
    url: String,
    payload_json: Option<Value>,
    response_json: Option<Value>,
    created_at: DateTime<Utc>,
}

pub struct UiDisplayRequest {
    pub key: String,
    pub content: Option<Value>,
    pub response: Option<Value>,
}

pub fn to_ui_displayable(data: Vec<ScoobyRequest>) -> Vec<UiDisplayRequest> {
    data.iter()
        .map(|item| UiDisplayRequest {
            key: format!(
                "{} {} {} {} {}",
                item.method, item.service, item.route_url, item.created_at, item.route_url,
            ),
            content: item.payload_json.clone(),
            response: item.response_json.clone(),
        })
        .collect()
}

fn colored_json_opt(v: &Option<Value>) -> String {
    match v {
        None => "null".into(),
        Some(val) => to_colored_json_auto(val).unwrap_or_else(|_| "<invalid json>".into()),
    }
}

impl Display for ScoobyRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "[{}] {} {} {}",
            self.created_at,
            self.method.purple(),
            self.service.green(),
            self.url.yellow()
        )?;

        writeln!(f, "  payload: {}", colored_json_opt(&self.payload_json))?;
        writeln!(f, "  response: {}", colored_json_opt(&self.response_json))?;

        Ok(())
    }
}

fn parse_json_opt(s: Option<String>) -> Result<Option<Value>, serde_json::Error> {
    match s {
        None => Ok(None),
        Some(txt) => {
            let v = from_str(&txt).map(Some)?;
            Ok(v)
        }
    }
}

impl Db {
    async fn setup_tables(&self) -> Result<(), Box<dyn Error>> {
        const SQL_STR: &str = "CREATE TABLE IF NOT EXISTS requests (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        method TEXT NOT NULL,
        service TEXT NOT NULL,
        route_url TEXT NOT NULL,
        full_url TEXT NOT NULL,
        payload TEXT,
        response_json TEXT,
        created_at TEXT NOT NULL)";

        match self.conn.execute(SQL_STR, ()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                eprintln!("Couldn't setup tables: {}", err);
                std::process::exit(1)
            }
        }
    }

    pub async fn create_connection() -> Result<Self, Box<dyn Error>> {
        const LOCAL_DB_DIR_PATH: &str = ".scooby";
        let local_db_path = format!("{}/{}", std::env::var("HOME")?, LOCAL_DB_DIR_PATH);
        if !Path::new(&local_db_path).exists() {
            fs::create_dir_all(local_db_path.clone())
                .await
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed to create scooby db directory at {}: {}",
                        local_db_path, err
                    )
                })
        }

        let full_path = format!("{}/{}", local_db_path, "dooby.db");
        let db = match Builder::new_local(&full_path).build().await {
            Ok(db) => db,
            Err(err) => panic!("Local database connection failed with: {}", err),
        };

        let conn = match db.connect() {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("Couldn't establish connection to db: {}", err);
                std::process::exit(1)
            }
        };

        let db = Self { conn };

        let _ = db.setup_tables().await;

        Ok(db)
    }

    pub async fn insert_args(
        &self,
        store_args: DbStoreArgs,
        res: ResponseParts,
    ) -> Result<(), Box<dyn Error>> {
        const SQL_STR: &str = "INSERT INTO requests (
        method,
        service,
        route_url,
        full_url,
        payload,
        response_json,
        created_at
    ) VALUES (?, ?, ?, ?, ?, ?, ?)";

        let payload_json_string = serde_json::to_string(&store_args.payload)?;
        let response_json_string = serde_json::to_string(&res.body)?;

        let now: DateTime<Utc> = SystemTime::now().into();
        let created_at = now.to_rfc3339();

        self.conn
            .execute(
                SQL_STR,
                (
                    store_args.method,
                    store_args.service,
                    store_args.route_url,
                    store_args.url,
                    payload_json_string,
                    response_json_string,
                    created_at,
                ),
            )
            .await
            .unwrap_or_else(|_| panic!("{}", "Ruh roh, couldn't store values in db!".red()));

        Ok(())
    }

    pub async fn get_all_entries(&self) -> Result<Vec<ScoobyRequest>, Box<dyn Error>> {
        const SQL_STR: &str = "SELECT * FROM requests";

        let mut rows = self.conn.query(SQL_STR, [1]).await?;
        let mut output = Vec::new();

        while let Some(row) = rows.next().await? {
            let data = match Db::map_to_domain(row).await {
                Ok(mapped) => mapped,
                Err(err) => {
                    panic!("Something went wrong mapping data to domain: {}", err)
                }
            };
            output.push(data);
        }

        Ok(output)
    }

    pub async fn get_all_entries_by_time_range(
        &self,
        time: DateTime<Utc>,
    ) -> Result<Vec<ScoobyRequest>, Box<dyn Error>> {
        const SQL_STR: &str =
            "SELECT * FROM requests WHERE created_at > ?1 ORDER BY created_at ASC";

        let since = time.to_rfc3339();

        let mut rows = self.conn.query(SQL_STR, [since]).await?;
        let mut output = Vec::new();

        while let Some(row) = rows.next().await? {
            let data = match Db::map_to_domain(row).await {
                Ok(mapped) => mapped,
                Err(err) => {
                    panic!("Something went wrong mapping data to domain: {}", err)
                }
            };
            output.push(data);
        }

        Ok(output)
    }

    pub async fn get_all_entries_by_service(
        &self,
        service: String,
        time: DateTime<Utc>,
    ) -> Result<Vec<ScoobyRequest>, Box<dyn Error>> {
        const SQL_STR: &str =
            "SELECT * FROM requests WHERE service = ?1 AND created_at > ?2 ORDER BY created_at ASC";

        let since = time.to_rfc3339();

        let mut rows = self.conn.query(SQL_STR, [service, since]).await?;
        let mut output = Vec::new();

        while let Some(row) = rows.next().await? {
            let data = match Db::map_to_domain(row).await {
                Ok(mapped) => mapped,
                Err(err) => {
                    panic!("Something went wrong mapping data to domain: {}", err)
                }
            };
            output.push(data);
        }

        Ok(output)
    }

    async fn map_to_domain(row: Row) -> Result<ScoobyRequest, Box<dyn Error>> {
        let method: String = row.get(1)?;
        let service: String = row.get(2)?;
        let route_url: String = row.get(3)?;
        let url: String = row.get(4)?;

        let payload_text: Option<String> = row.get(5)?;
        let payload_json = parse_json_opt(payload_text)?;

        let response_text: Option<String> = row.get(6)?;
        let response_json = parse_json_opt(response_text)?;

        let created_at_text: String = row.get(7)?;
        let created_at = created_at_text.parse::<DateTime<Utc>>()?;

        Ok(ScoobyRequest {
            method,
            service,
            route_url,
            url,
            payload_json,
            response_json,
            created_at,
        })
    }
}
