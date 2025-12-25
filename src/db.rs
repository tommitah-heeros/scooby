use chrono::{DateTime, Utc};
use colored::Colorize;
use colored_json::to_colored_json_auto;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str};
use std::{env, error::Error, fmt::Display, path::Path, time::SystemTime};
use tokio::fs;
use turso::{Builder, Connection};

use crate::http::ResponseParts;

// todo: for now this is a "C-style" library file, but should probably refactor it into a proper
// db layer.

pub async fn create_db_connection() -> Result<Connection, Box<dyn Error>> {
    let db_dir = env::var("scooby_db_path")
        .expect("Define local database path in shell config (export scooby_db_path=\"...\")");

    if !Path::new(&db_dir).exists() {
        fs::create_dir_all(&db_dir).await.unwrap_or_else(|e| {
            panic!("Failed to create scooby db directory at {}: {}", db_dir, e)
        });
    }

    let full_path = format!("{}/{}", db_dir, "dooby.db");
    let db = Builder::new_local(&full_path)
        .build()
        .await
        .expect("Something went wrong initializing turso for scooby.");

    let conn = db
        .connect()
        .expect("Something went wrong making a connection to database.");

    Ok(conn)
}

pub async fn setup_tables(conn: &Connection) -> Result<(), Box<dyn Error>> {
    const SQL_STR: &str = "CREATE TABLE IF NOT EXISTS requests (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        method TEXT NOT NULL,
        service TEXT NOT NULL,
        route_url TEXT NOT NULL,
        full_url TEXT NOT NULL,
        payload TEXT,
        response_json TEXT,
        created_at TEXT NOT NULL
    )";

    conn.execute(SQL_STR, ())
        .await
        .expect("Couldn't setup tables.");

    println!("{}", "Table setup complete!".green());

    Ok(())
}

pub struct DbStoreArgs {
    pub method: String,
    pub service: String,
    pub url: String,
    pub route_url: String,
    pub payload: Option<serde_json::Value>,
}

pub async fn store_run_into_db(
    conn: &Connection,
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

    let payload_json_string = serde_json::to_string(&store_args.payload)
        .expect("Couldn't parse json value back into string");
    let response_json_string =
        serde_json::to_string(&res.body).expect("Couldn't parse json value back into string.");

    let now: DateTime<Utc> = SystemTime::now().into();
    let created_at = now.to_rfc3339();

    conn.execute(
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
    .expect(&"Ruh roh, couldn't store values in db!".red());

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    method: String,
    service: String,
    route_url: String,
    url: String,
    payload_json: Option<Value>,
    response_json: Option<Value>,
    created_at: DateTime<Utc>,
}

fn colored_json_opt(v: &Option<Value>) -> String {
    match v {
        None => "null".into(),
        Some(val) => to_colored_json_auto(val).unwrap_or_else(|_| "<invalid json>".into()),
    }
}

impl Display for Request {
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

pub async fn get_all_entries_by_time_range(
    conn: &Connection,
    time: DateTime<Utc>,
) -> Result<Vec<Request>, Box<dyn Error>> {
    const SQL_STR: &str = "SELECT * FROM requests WHERE created_at > ?1 ORDER BY created_at ASC";

    let since = time.to_rfc3339();

    let mut rows = conn.query(SQL_STR, [since]).await?;
    let mut output = Vec::new();

    // something tells me this is not a safe way to do this...
    while let Some(row) = rows.next().await? {
        let method: String = row.get(1)?;
        let service: String = row.get(2)?;
        let route_url: String = row.get(3)?;
        let url: String = row.get(4)?;

        let payload_text: Option<String> = row.get(5)?;
        let payload_json = parse_json_opt(payload_text)?;

        let response_text: Option<String> = row.get(6)?;
        let response_json = parse_json_opt(response_text)?;

        let created_at_text: String = row.get(7)?;
        let created_at = created_at_text
            .parse::<DateTime<Utc>>()
            .expect("Db data deserialization failed for created_at");

        output.push(Request {
            method,
            service,
            route_url,
            url,
            payload_json,
            response_json,
            created_at,
        })
    }

    Ok(output)
}
