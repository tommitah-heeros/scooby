use clap::{Args, Parser, Subcommand, ValueEnum};
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ServerEnv {
    Dev,
    Test,
    Prod,
}

impl AsRef<str> for ServerEnv {
    fn as_ref(&self) -> &str {
        match self {
            ServerEnv::Dev => "dev",
            ServerEnv::Test => "test",
            ServerEnv::Prod => "cloud",
        }
    }
}

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct ScoobyArgs {
    #[clap(subcommand)]
    pub mode_type: ModeType,
}

#[derive(Debug, Subcommand)]
pub enum ModeType {
    /// Create and process HTTP requests to Modular services
    Req(ReqCommand),

    /// Query, view and export previous requests.
    #[clap(subcommand)]
    Ask(AskCommand),
}

#[derive(Debug, Args)]
pub struct ReqCommand {
    // for now, use the simple version
    /// HTTP Method
    #[arg(value_enum)]
    pub method: Method, // todo: this might just be a subcommand, so we can have separate args for different methods.

    /// Which Modular service to query
    #[arg(value_enum)]
    pub service: ModularService,

    /// Resource route
    #[arg()]
    pub route_url: String,

    /// Dev-stack prefix, defaults to "tommitah-"
    #[arg(short('d'), long("dev"), default_value("tommitah-"))]
    pub dev_prefix: String,

    /// Server environment, defaults to dev
    #[arg(short('s'), long("server"), value_enum, default_value = "dev")]
    pub server_env: ServerEnv,

    /// Querystring parameters
    #[arg(short, long)]
    pub qsp: Option<String>,

    /// Where to look for json payload
    #[arg(
        short('p'),
        long("payload"),
        required_if_eq("method", "POST"),
        required_if_eq("method", "PATCH")
    )]
    pub payload_path: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListAllCommand {
    #[arg()]
    pub time_range: String,
}

#[derive(Debug, Args)]
pub struct ListByServiceCommand {
    #[arg()]
    pub time_range: String,
}

#[derive(Debug, Subcommand)]
pub enum AskCommand {
    /// List all requests made
    ListAll(ListAllCommand),

    /// List all requests made to a specific service
    ListByService(ListByServiceCommand),
}

// Instead of this approach, we should prefer deriving clap::Parser and just using it straight
// in the main entry.
//
// Also the struct extending seems to be a lot cleaner and safer for the consuming code.
