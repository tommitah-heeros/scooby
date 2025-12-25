mod cli;
mod db;
mod formatting;
mod http;

use clap::Parser;
use colored::Colorize;
use std::env;
use tokio::fs;

use formatting::pretty_print_response;

use db::{create_db_connection, setup_tables, store_run_into_db};
use http::{HttpClientParams, create_http_client, split_http_response};

use cli::{AskCommand, ModeType, ReqCommand, ScoobyArgs};
use db::DbStoreArgs;

async fn handle_req_mode(cli: ReqCommand) {
    let db = create_db_connection()
        .await
        .expect(&"Ruh roh, DB startup pooped.".red());

    setup_tables(&db)
        .await
        .expect(&"Ruh roh, table setup pooped.".red());

    let auth_token = if env::var("ltpa_token").is_ok() {
        env::var("ltpa_token")
    } else {
        eprintln!("giff ltpa you bastard");
        std::process::exit(1);
    };

    let service_name = cli.service.as_ref();
    let service_url = format!("{}{}", cli.dev_prefix, service_name);

    let base_url = format!("https://api.{}.heeros.com/", cli.server_env.as_ref());

    let route = cli.route_url;

    let payload_path = cli.payload_path;

    let url = format!(
        "{}{}/{}{}",
        base_url,
        service_url,
        route,
        cli.qsp.unwrap_or_else(String::new)
    );
    println!("\nRequesting: {}\n", url.purple());

    let http_params = HttpClientParams {
        timeout_secs: 15,
        token: auth_token.expect("required"), // oof...
    };
    let http_client = create_http_client(http_params);

    let mut req_builder = http_client.request(cli.method.clone(), url.clone());
    let mut json_payload: Option<serde_json::Value> = None;

    if let Some(path) = payload_path {
        let payload = fs::read_to_string(path)
            .await
            .expect("Expected a valid error path.");

        let json: serde_json::Value =
            serde_json::from_str(&payload).expect(&"JSON payload not correctly formatted!".red());
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
                route_url: route,
                payload: json_payload,
            };
            store_run_into_db(&db, db_store_args, parts)
                .await
                .expect(&"Ruh roh, couldn't insert run to db!".red());

            ()
        }
        Err(_) => panic!("Ruh roh, request errored!"),
    }
}

async fn handle_ask_mode(_cli: AskCommand) {
    todo!("this is not implemented yet!");
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = ScoobyArgs::parse();

    match args.mode_type {
        ModeType::Req(cli) => {
            handle_req_mode(cli).await;
        }
        ModeType::Ask(cli) => {
            handle_ask_mode(cli).await;
        }
    }

    Ok(())
}
