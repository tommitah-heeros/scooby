use colored::{Color, Colorize};
use colored_json::to_colored_json_auto;

use crate::http::ResponseParts;

pub async fn pretty_print_response(parts: &ResponseParts) {
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
        println!(
            "{}: {}",
            key.as_str().cyan(),
            value.to_str().unwrap_or_default().white()
        );
    }

    println!();

    let colored_output = match to_colored_json_auto(&parts.body) {
        Ok(some) => some,
        Err(err) => {
            eprintln!("Couldn't produce pretty output: {}", err);
            std::process::exit(1)
        }
    };

    println!("{}", colored_output);
}
