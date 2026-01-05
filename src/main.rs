#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]

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
    let db = match Db::create_connection().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("{}: {}", Colorize::red("Ruh roh, db isn't working"), err);
            eprintln!("Exiting...");
            std::process::exit(1)
        }
    };

    // service url parts are stored in config data, user gets to choose the option to use
    let service_name = cfg.get(&cli.service);
    let service_url = format!("{}{}", cfg.get(&cli.dev_prefix), service_name);
    let domain_url = cfg.get(&cli.domain_url);

    let base_url = domain_url.replace("[SERVER_ENV]", cli.server_env.as_ref());

    let url = format!(
        "{}{}/{}{}",
        base_url,
        service_url,
        cli.route_url,
        cli.qsp.unwrap_or_default()
    );
    println!("\nRequesting: {}\n", url.purple());

    // longish timeout, the apis are quite slow sometimes...
    let timeout_secs: u64 = 15;
    let http_client = create_http_client(timeout_secs);

    let mut req_builder = http_client.request(cli.method.clone(), url.clone());
    let mut json_payload: Option<serde_json::Value> = None;

    if let Some(path) = cli.payload_path {
        let payload = match fs::read_to_string(path).await {
            Ok(payload) => payload,
            Err(err) => {
                eprintln!("No (valid) json payload in provided path: {}", err);
                std::process::exit(1)
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&payload) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("Couldn't read json from payload: {}", err);
                std::process::exit(1);
            }
        };

        req_builder = req_builder.json(&json);
        json_payload = Some(json);
    }

    let response = req_builder.send().await;

    match response {
        Ok(res) => {
            let parts = match split_http_response(res).await {
                Ok(parts) => parts,
                Err(err) => {
                    eprintln!("Couldn't parse response: {}", err);
                    std::process::exit(1)
                }
            };

            pretty_print_response(&parts).await;

            let db_store_args = DbStoreArgs {
                method: cli.method.to_string(),
                service: service_name.to_string(),
                url,
                route_url: cli.route_url,
                payload: json_payload,
            };

            match db.insert_args(db_store_args, parts).await {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Inserting data to db failed: {}", err);
                    std::process::exit(1)
                }
            };
        }
        Err(err) => {
            eprintln!("{}: {}", Colorize::red("Ruh roh, request errored!"), err);
            eprintln!("Exiting...");
            std::process::exit(1)
        }
    }
}

fn date_to_utc_start(s: String) -> Result<DateTime<Utc>, chrono::ParseError> {
    let date = NaiveDate::parse_from_str(s.as_str(), "%Y-%m-%d")?;
    let date_time =
        DateTime::from_naive_utc_and_offset(date.and_hms_opt(0, 0, 0).unwrap_or_default(), Utc);
    Ok(date_time)
}

async fn handle_db_mode(cli: DbCommand, cfg: Cfg) {
    let db = match Db::create_connection().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("{}: {}", Colorize::red("Ruh roh, db isn't working"), err);
            eprintln!("Exiting...");
            std::process::exit(1)
        }
    };

    match cli {
        DbCommand::ListAll(cli) => {
            let date_time = if let Ok(date_time) = date_to_utc_start(cli.time_range) {
                date_time
            } else {
                eprintln!("Something went wrong parsing date input");
                eprintln!("Exiting...");
                std::process::exit(1)
            };

            let list = match db.get_all_entries_by_time_range(date_time).await {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("Couldn't query all the entries: {}", err);
                    std::process::exit(1)
                }
            };

            for entry in list {
                println!("{}", entry);
            }
        }
        DbCommand::ListByService(cli) => {
            let date_time = if let Ok(date_time) = date_to_utc_start(cli.time_range) {
                date_time
            } else {
                eprintln!("Something went wrong parsing date input");
                std::process::exit(1)
            };

            let list = match db
                .get_all_entries_by_service(cfg.get(&cli.service), date_time)
                .await
            {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("Couldn't query all the entries: {}", err);
                    std::process::exit(1)
                }
            };

            for entry in list {
                println!("{}", entry)
            }
        }
        DbCommand::Ui(_cli) => {
            let _ = Ui::run(&db);
        }
    };
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = ScoobyArgs::parse();

    let cfg = Cfg::parse_from_file();

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
