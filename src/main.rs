use colored::Colorize;
use reqwest::Method;
use std::env;
use tokio::fs;

mod cli;
use crate::cli::{ModularService, parse_cli_input};

mod formatting;
use crate::formatting::pretty_print_response;

mod http;
use crate::http::{HttpClientParams, create_http_client, split_http_response};

mod db;
use crate::db::{create_db_connection, setup_tables, store_run_into_db};

// this whole thing should probably be refactored with no `expect`'s and no `unwrap`'s
// Also would be nice to figure out if the Cli part is supposed to be wrapped at all and what is
// the safest way to make sure the types are correct at compile time.

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let db = create_db_connection()
        .await
        .expect(&"Ruh roh, DB startup pooped.".red());

    setup_tables(&db)
        .await
        .expect(&"Ruh roh, table setup pooped.".red());

    let args = parse_cli_input().get_matches();

    let args_method = args.get_one::<Method>("method").expect("required");
    let auth_token = if env::var("ltpa_token").is_ok() {
        env::var("ltpa_token")
    } else {
        eprintln!("giff ltpa you bastard");
        std::process::exit(1);
    };

    let args_service = args.get_one::<ModularService>("service").expect("required");
    let args_prefix = args.get_one::<String>("dev_prefix");
    let service_url = match args_prefix {
        None => String::from(args_service.as_ref()),
        Some(stack_prefix) => {
            let ser_ref = args_service.as_ref();
            format!("{stack_prefix}{ser_ref}")
        }
    };

    let args_server_env = args.get_one::<String>("server_env");
    let args_server_env = match args_server_env {
        Some(value) => value.clone(), // oof
        _ => String::from("dev"),
    };
    let base_url = format!("https://api.{args_server_env}.heeros.com/");

    let args_route_url = args.get_one::<String>("route_url").expect("required");

    let args_qsp_url = args.get_one::<String>("qsp");
    let qsp_url = match args_qsp_url {
        Some(value) => value.clone(), // oof
        _ => String::from(""),
    };

    let args_payload_path = args.get_one::<String>("payload_path");

    let url = format!("{base_url}{service_url}/{args_route_url}{qsp_url}");
    println!("\nRequesting: {}\n", url.purple());

    let http_params = HttpClientParams {
        timeout_secs: 15,
        token: auth_token.expect("required"), // oof...
    };
    let http_client = create_http_client(http_params);

    let mut req_builder = http_client.request(args_method.clone(), url);

    if let Some(payload_path) = args_payload_path {
        let payload = fs::read_to_string(payload_path)
            .await
            .expect("Expected a valid error path.");

        let json: serde_json::Value =
            serde_json::from_str(&payload).expect(&"JSON payload not correctly formatted!".red());
        req_builder = req_builder.json(&json);
    }

    let res = req_builder.send().await?;

    let parts = split_http_response(res)
        .await
        .expect("Ruh roh, couldn't split response.");

    pretty_print_response(&parts)
        .await
        .expect("Ruh roh, couldn't print results!");

    store_run_into_db(&db, args, parts)
        .await
        .expect(&"Ruh roh, couldn't insert run to db!".red());

    Ok(())
}
