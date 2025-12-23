use colored::{Color, Colorize};
use std::error::Error;

use crate::http::ResponseParts;

fn print_colorized_json(pretty_json: &str) {
    for line in pretty_json.lines() {
        let mut colored_line = String::new();

        let mut in_string = false;
        for c in line.chars() {
            match c {
                '"' => {
                    in_string = !in_string;
                    colored_line.push_str(&c.to_string().green().to_string());
                }
                ':' if !in_string => {
                    colored_line.push_str(&c.to_string().white().to_string());
                }
                ',' if !in_string => {
                    colored_line.push_str(&c.to_string().white().to_string());
                }
                _ if in_string => {
                    colored_line.push_str(&c.to_string().green().to_string());
                }
                _ if c.is_numeric() => {
                    colored_line.push_str(&c.to_string().yellow().to_string());
                }
                _ => {
                    colored_line.push(c);
                }
            }
        }

        println!("{}", colored_line);
    }
}

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

    let pretty = serde_json::to_string_pretty(&parts.body).expect("Output json format incorrect.");
    println!("{:?}", print_colorized_json(&pretty));

    Ok(())
}
