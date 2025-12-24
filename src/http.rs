use reqwest::{Client, Response, StatusCode, header::HeaderMap};
use serde_json::Value;
use std::error::Error;

pub struct ResponseParts {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

pub struct HttpClientParams {
    pub timeout_secs: u64,
    pub token: String,
}
pub fn create_http_client(params: HttpClientParams) -> Client {
    let token = params.token;
    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(params.timeout_secs))
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Cookie", format!("LtpaToken={token}").parse().unwrap());
            headers
        })
        .build();

    match http_client {
        Ok(inst) => inst,
        Err(_) => panic!("Http client instance not available."),
    }
}

pub async fn split_http_response(res: Response) -> Result<ResponseParts, Box<dyn Error>> {
    let status = res.status();
    let headers = res.headers().clone();
    let body: Value = res.json().await.expect("Output json format was incorrect.");

    Ok(ResponseParts {
        status,
        headers,
        body,
    })
}
