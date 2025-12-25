use colored::{Color, Colorize};
use colored_json::to_colored_json_auto;
use std::error::Error;

use crate::http::ResponseParts;

pub async fn pretty_print_response(parts: &ResponseParts) -> Result<(), Box<dyn Error>> {
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

    println!("{}", to_colored_json_auto(&parts.body)?);

    Ok(())
}
