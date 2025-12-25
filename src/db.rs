use chrono::{DateTime, Utc};
use colored::Colorize;
use std::{env, error::Error, path::Path, time::SystemTime};
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

pub async fn setup_tables(db: &Connection) -> Result<(), Box<dyn Error>> {
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

    db.execute(SQL_STR, ())
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
    db: &Connection,
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

    db.execute(
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
