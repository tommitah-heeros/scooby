use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;

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
    Db(DbCommand),
}

#[derive(Debug, Args)]
pub struct ReqCommand {
    // for now, use the simple version
    /// HTTP Method
    #[arg(value_enum)]
    pub method: Method, // todo: this might just be a subcommand, so we can have separate args for different methods.

    /// Target service. Intended use is to use an abbreviation which is linked to a
    /// value in `config.toml`: `scooby req GET <my-abbr> some-resource/some-id`.
    /// config.toml: my-abbr = "some-longer-part-of-url"
    #[arg()]
    pub service: String,

    /// Resource route
    #[arg()]
    pub route_url: String,

    /// Dev-stack prefix, defaults to "tommitah-"
    #[arg(short('d'), long("dev"), default_value(""))]
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
    pub service: String,
    #[arg()]
    pub time_range: String,
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// List all requests made
    ListAll(ListAllCommand),

    /// List all requests made to a specific service
    ListByService(ListByServiceCommand),
}
