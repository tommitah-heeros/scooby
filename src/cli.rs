use clap::{Parser, ValueEnum};
use reqwest::Method;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ModularService {
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
pub struct Cli {
    //// MANDATORY ARGS w/o flags
    #[arg(value_enum)]
    pub method: Method,

    #[arg(value_enum)]
    pub service: ModularService,

    #[arg()]
    pub route_url: String,

    #[arg()]
    pub say_it: Option<String>,

    //// OPTIONAL ARGS w/ flags
    #[arg(short, long)]
    pub dev_prefix: Option<bool>,

    #[arg(short, long)]
    pub ltpa: Option<String>,

    #[arg(short, long)]
    pub server_env: Option<String>,

    #[arg(short, long)]
    pub qsp: Option<String>,
    #[arg(short, long)]
    pub payload_path: Option<String>,

    #[arg(short, long)]
    pub flush_storage: Option<bool>,
}
