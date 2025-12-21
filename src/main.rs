use std::env;

use clap::{Parser, ValueEnum};
use reqwest::{Client, Method, header::HeaderMap};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ModularService {
    HPI,
    HSI,
    CR,
    CIS,
}

fn match_service(name: &ModularService) -> &str {
    match name {
        ModularService::HPI => "windmill-service-v1",
        ModularService::HSI => "sales-invoice-service-v1",
        ModularService::CR => "cloudreader-v1",
        ModularService::CIS => "circula-integration-service-v1",
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    //// MANDATORY ARGS w/o flags
    #[arg(value_enum)]
    method: Method,

    #[arg(value_enum)]
    service: ModularService,

    #[arg()]
    route_url: String,

    #[arg()]
    say_it: Option<String>,

    //// OPTIONAL ARGS w/ flags
    #[arg(short, long)]
    dev_prefix: Option<bool>,

    #[arg(short, long)]
    ltpa: Option<String>,

    #[arg(short, long)]
    server_env: Option<String>,

    #[arg(short, long)]
    qsp: Option<String>,

    #[arg(short, long)]
    payload: Option<String>, // todo: file name for a "body" json file
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = Cli::parse();

    let ltpa_token = args
        .ltpa
        .or_else(|| env::var("ltpa_token").ok())
        .unwrap_or_else(|| {
            eprintln!("Giff ltpa you bastard");
            std::process::exit(1);
        });

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .default_headers({
            let mut headers = HeaderMap::new();
            headers.insert("Cookie", format!("LtpaToken={ltpa_token}").parse().unwrap());
            headers
        })
        .build()?;

    let service_name = match_service(&args.service);
    let service_url = match args.dev_prefix {
        Some(false) => String::from(service_name),
        _ => format!("tommitah-{service_name}"),
    };

    let server_env = match args.server_env {
        Some(value) => value,
        _ => String::from("dev"),
    };
    let base_url = format!("https://api.{server_env}.heeros.com/");

    let resource_url = &args.route_url;
    let qsp_url = match args.qsp {
        Some(value) => value,
        _ => String::from(""),
    };

    let url = format!("{base_url}{service_url}/{resource_url}{qsp_url}");

    let res = http_client
        .request(args.method, url)
        .send()
        .await?
        .text()
        .await?;

    println!("{}", res);

    // scooby post hpi "/invoices" -p files/my_payload.json
    // HTTP request POST https://api.{env}.heeros.com/tommitah-{hpi}{"/invoices"} -b {...payload...}

    if let Some(say_it) = args.say_it.as_deref() {
        println!("Ruh roh {}, where are my rhesticles?!", say_it);
    }

    Ok(())
}
