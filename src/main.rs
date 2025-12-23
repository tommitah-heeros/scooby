use std::{env, error::Error, path::Path};

use clap::{Parser, ValueEnum};
use colored::{Color, Colorize};
use reqwest::{Client, Method, Response};

use tokio::fs;
use turso::{Builder, Connection};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ModularService {
    HPI,
    HSI,
    CR,
    CIS,
}

impl AsRef<str> for ModularService {
    fn as_ref(&self) -> &str {
        match self {
            ModularService::HPI => "windmill-service-v1",
            ModularService::HSI => "sales-invoice-service-v1",
            ModularService::CR => "cloudreader-v1",
            ModularService::CIS => "circula-integration-service-v1",
        }
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
    payload_path: Option<String>,

    #[arg(short, long)]
    flush_storage: Option<bool>,
}

struct ResponseParts {
    status: reqwest::StatusCode,
    headers: reqwest::header::HeaderMap,
    body: serde_json::Value,
}

async fn split_http_response(res: Response) -> Result<ResponseParts, Box<dyn Error>> {
    let status = res.status();
    let headers = res.headers().clone();
    let body: serde_json::Value = res.json().await.expect("Output json format was incorrect.");

    Ok(ResponseParts {
        status,
        headers,
        body,
    })
}

fn print_colorized_json(pretty_json: &str) {
    for line in pretty_json.lines() {
        let mut colored_line = String::new();

        let mut in_string = false;
        for c in line.chars() {
            match c {
                '"' => {
                    in_string = !in_string;
                    colored_line.push_str(&c.to_string().green().to_string());
                }
                ':' if !in_string => {
                    colored_line.push_str(&c.to_string().white().to_string());
                }
                ',' if !in_string => {
                    colored_line.push_str(&c.to_string().white().to_string());
                }
                _ if in_string => {
                    colored_line.push_str(&c.to_string().green().to_string());
                }
                _ if c.is_numeric() => {
                    colored_line.push_str(&c.to_string().yellow().to_string());
                }
                _ => {
                    colored_line.push(c);
                }
            }
        }

        println!("{}", colored_line);
    }
}

async fn pretty_print_response(parts: &ResponseParts) -> Result<(), Box<dyn Error>> {
    let status_color = if parts.status.is_success() {
        Color::Green
    } else if parts.status.is_client_error() {
        Color::Yellow
    } else {
        Color::Red
    };

    println!(
        "{} {}",
        "HTTP/1.1".bold(),
        parts.status.to_string().color(status_color).bold()
    );

    for (key, value) in parts.headers.iter() {
        println!("{}: {}", key.as_str().cyan(), value.to_str()?.white());
    }

    println!();

    let pretty = serde_json::to_string_pretty(&parts.body).expect("Output json format incorrect.");
    println!("{:?}", print_colorized_json(&pretty));

    Ok(())
}

async fn create_db_connection() -> Result<Connection, Box<dyn Error>> {
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

async fn setup_tables(db: &Connection) -> Result<(), Box<dyn Error>> {
    const SQL_STR: &str = "CREATE TABLE IF NOT EXISTS requests (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        method TEXT NOT NULL,
        service TEXT NOT NULL,
        route_url TEXT NOT NULL,
        full_url TEXT NOT NULL,
        response_json TEXT,
        created_at TEXT NOT NULL
    )";

    db.execute(SQL_STR, ())
        .await
        .expect("Couldn't setup tables.");

    println!("{}", "Table setup complete!".green());

    Ok(())
}

async fn store_run_into_db(
    db: &Connection,
    cli_args: Cli,
    res: ResponseParts,
) -> Result<(), Box<dyn Error>> {
    const SQL_STR: &str = "INSERT INTO requests (
        method,
        service,
        route_url,
        full_url,
        response_json,
        created_at
    ) VALUES (?, ?, ?, ?, ?, ?)";

    let json_string =
        serde_json::to_string(&res.body).expect("Couldn't parse json value back into string.");

    let now: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    let created_at = now.to_rfc3339();

    db.execute(
        SQL_STR,
        (
            cli_args.method.as_str(),
            cli_args.service.as_ref(),
            String::from(cli_args.route_url),
            "full_url",
            json_string,
            created_at,
        ),
    )
    .await
    .expect(&"Ruh roh, couldn't store values in db!".red());

    Ok(())
}

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
