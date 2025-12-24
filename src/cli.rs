use clap::{Arg, Command, ValueEnum};
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

pub fn parse_cli_input() -> Command {
    Command::new("Scooby HTTP Service Client")
        .version("0.1.0")
        .about("Performs http queries in the Modular environment")
        .arg(
            Arg::new("method")
                .value_parser(clap::value_parser!(Method))
                .required(true),
        )
        .arg(
            Arg::new("service")
                .value_parser(clap::value_parser!(ModularService))
                .required(true),
        )
        .arg(Arg::new("route_url").required(true))
        .arg(
            Arg::new("dev_prefix")
                .short('d')
                .long("dev_prefix")
                .default_value("tommitah-"),
        )
        // .arg(
        //     Arg::new("header_auth_token")
        //         .short('t')
        //         .long("auth_token")
        //         .value_parser(clap::value_parser!(Option<String>)),
        // )
        .arg(Arg::new("server_env").short('e').long("env"))
        .arg(Arg::new("qsp").short('q').long("qsp"))
        .arg(
            Arg::new("payload_path")
                .short('p')
                .long("payload_path")
                .required_if_eq("method", "POST"),
        )
    // .arg(
    //     Arg::new("flush_storage")
    //         .short('f')
    //         .long("flush")
    //         .value_parser(clap::value_parser!(bool))
    //         .default_value(false),
    // )
}
