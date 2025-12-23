use reqwest::Response;
use std::error::Error;

pub struct ResponseParts {
    pub status: reqwest::StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: serde_json::Value,
}

pub async fn split_http_response(res: Response) -> Result<ResponseParts, Box<dyn Error>> {
    let status = res.status();
    let headers = res.headers().clone();
    let body: serde_json::Value = res.json().await.expect("Output json format was incorrect.");

    Ok(ResponseParts {
        status,
        headers,
        body,
    })
}
