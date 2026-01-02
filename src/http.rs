use reqwest::{Client, Response, StatusCode, header::HeaderMap};
use serde_json::Value;
use std::error::Error;

pub struct ResponseParts {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

pub fn create_http_client(timeout_secs: u64) -> Client {
    let cookie_value = match std::env::var("auth_token") {
        Ok(token) => token,
        Err(err) => {
            eprintln!("No auth token found, couldn't construct headers {}", err);
            std::process::exit(1)
        }
    };

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            if let Ok(parsed_value) = cookie_value.parse() {
                headers.insert("Cookie", parsed_value);
            }
            headers
        })
        .build();

    match http_client {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Couldn't construct an http client instance {}", err);
            std::process::exit(1)
        }
    }
}

pub async fn split_http_response(res: Response) -> Result<ResponseParts, Box<dyn Error>> {
    let status = res.status();
    let headers = res.headers().clone();
    let body: serde_json::Value = res.json().await?;

    Ok(ResponseParts {
        status,
        headers,
        body,
    })
}
