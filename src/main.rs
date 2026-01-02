mod cfg;
mod cli;
mod db;
mod formatting;
mod http;
mod ui;

use chrono::{DateTime, NaiveDate, Utc};
use clap::Parser;
use colored::Colorize;
use tokio::fs;

use formatting::pretty_print_response;

use cfg::Cfg;
use db::{Db, DbStoreArgs};
use http::{create_http_client, split_http_response};

use cli::{DbCommand, ModeType, ReqCommand, ScoobyArgs};

use ui::Ui;

async fn handle_req_mode(cli: ReqCommand, cfg: Cfg) {
    let db_path = std::env::var("scooby_db_path")
        .expect("Define local database path in shell config (export scooby_db_path=\"...\")");
    let db = match Db::create_connection(db_path).await {
        Ok(db) => db,
        Err(err) => panic!("{}: {}", Colorize::red("Ruh roh, db isn't working"), err),
    };

    // service url parts are stored in config data, user gets to choose the option to use
    let service_name = cfg.get(&cli.service);
    let service_url = format!("{}{}", cfg.get(&cli.dev_prefix), service_name);

    let base_url = format!("https://api.{}.heeros.com/", cli.server_env.as_ref());

    let url = format!(
        "{}{}/{}{}",
        base_url,
        service_url,
        cli.route_url,
        cli.qsp.unwrap_or_else(String::new)
    );
    println!("\nRequesting: {}\n", url.purple());

    // longish timeout, the apis are quite slow sometimes...
    let timeout_secs: u64 = 15;
    let http_client = create_http_client(timeout_secs);

    let mut req_builder = http_client.request(cli.method.clone(), url.clone());
    let mut json_payload: Option<serde_json::Value> = None;

    if let Some(path) = cli.payload_path {
        let payload = fs::read_to_string(path)
            .await
            .expect("Expected a valid error path.");

        let json: serde_json::Value = serde_json::from_str(&payload)
            .expect(&Colorize::red("JSON payload not correctly formatted!"));
        req_builder = req_builder.json(&json);
        json_payload = Some(json);
    }

    let response = req_builder.send().await;

    match response {
        Ok(res) => {
            let parts = split_http_response(res)
                .await
                .expect("Ruh roh, couldn't split response.");

            pretty_print_response(&parts)
                .await
                .expect("Ruh roh, couldn't print results!");

            let db_store_args = DbStoreArgs {
                method: cli.method.to_string(),
                service: service_name.to_string(),
                url,
                route_url: cli.route_url,
                payload: json_payload,
            };
            db.insert_args(db_store_args, parts)
                .await
                .expect(&Colorize::red("Ruh roh, couldn't insert run to db!"));

            ()
        }
        Err(_) => panic!("Ruh roh, request errored!"),
    }
}

fn date_to_utc_start(s: String) -> Result<DateTime<Utc>, chrono::ParseError> {
    let date = NaiveDate::parse_from_str(s.as_str(), "%Y-%m-%d")?;
    let date_time = DateTime::from_naive_utc_and_offset(date.and_hms_opt(0, 0, 0).unwrap(), Utc);
    Ok(date_time)
}

async fn handle_db_mode(cli: DbCommand, cfg: Cfg) {
    let db_path = std::env::var("scooby_db_path")
        .expect("Define local database path in shell config (export scooby_db_path=\"...\")");
    let db = match Db::create_connection(db_path).await {
        Ok(db) => db,
        Err(err) => panic!("{}: {}", Colorize::red("Ruh roh, db isn't working"), err),
    };

    match cli {
        DbCommand::ListAll(cli) => {
            let date_time = if let Ok(date_time) = date_to_utc_start(cli.time_range) {
                date_time
            } else {
                panic!("Something went wrong parsing date input")
            };

            let list = db
                .get_all_entries_by_time_range(date_time)
                .await
                .expect("bar");

            for entry in list {
                println!("{}", entry);
            }
        }
        DbCommand::ListByService(cli) => {
            let date_time = if let Ok(date_time) = date_to_utc_start(cli.time_range) {
                date_time
            } else {
                panic!("Something went wrong parsing date input")
            };

            let list = db
                .get_all_entries_by_service(cfg.get(&cli.service), date_time)
                .await
                .expect("Data entries");

            for entry in list {
                println!("{}", entry)
            }
        }
        DbCommand::Ui(_cli) => {
            let _ = Ui::run();
        }
    };
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = ScoobyArgs::parse();

    let home_dir = std::env::var("HOME").unwrap();
    let file_path = format!("{home_dir}/.config/scooby/config");

    let cfg = Cfg::parse_from_file(file_path);

    match args.mode_type {
        ModeType::Req(cli) => {
            handle_req_mode(cli, cfg).await;
        }
        ModeType::Db(cli) => {
            handle_db_mode(cli, cfg).await;
        }
    }

    Ok(())
}
