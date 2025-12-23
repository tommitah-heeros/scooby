use clap::Parser;
use colored::Colorize;
use reqwest::Client;
use std::env;
use tokio::fs;

mod cli;
use crate::cli::Cli;

mod formatting;
use crate::formatting::pretty_print_response;

mod http;
use crate::http::split_http_response;

mod db;
use crate::db::{create_db_connection, setup_tables, store_run_into_db};

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let db = create_db_connection()
        .await
        .expect(&"Ruh roh, DB startup pooped.".red());

    setup_tables(&db)
        .await
        .expect(&"Ruh roh, table setup pooped.".red());

    let args = Cli::parse();

    let cli_ltpa_token = args.ltpa.clone();
    let ltpa_token = cli_ltpa_token
        .or_else(|| env::var("ltpa_token").ok())
        .unwrap_or_else(|| {
            eprintln!("Giff ltpa you bastard");
            std::process::exit(1);
        });

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Cookie", format!("LtpaToken={ltpa_token}").parse().unwrap());
            headers
        })
        .build()?;

    let service_name = args.service.as_ref();
    let service_url = match args.dev_prefix {
        Some(false) => String::from(service_name),
        _ => format!("tommitah-{service_name}"),
    };

    let cli_server_env = args.server_env.clone();
    let server_env = match cli_server_env {
        Some(value) => value,
        _ => String::from("dev"),
    };
    let base_url = format!("https://api.{server_env}.heeros.com/");

    let resource_url = args.route_url.clone();

    let cli_qsp_url = args.qsp.clone();
    let qsp_url = match cli_qsp_url {
        Some(value) => value,
        _ => String::from(""),
    };

    let url = format!("{base_url}{service_url}/{resource_url}{qsp_url}");
    println!("\nRequesting: {}\n", url.purple());

    let mut req_builder = http_client.request(args.method.clone(), url);

    if let Some(payload_path) = args.payload_path.as_deref() {
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

    if let Some(say_it) = args.say_it.as_deref() {
        println!("Ruh roh {}, where are my rhesticles?!", say_it);
    }

    store_run_into_db(&db, args, parts)
        .await
        .expect(&"Ruh roh, couldn't insert run to db!".red());

    Ok(())
}
