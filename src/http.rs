use reqwest::{Response, StatusCode, header::HeaderMap};
use serde_json::Value;
use std::error::Error;

pub struct ResponseParts {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
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
