use config::Config;
use std::collections::HashMap;

pub struct Cfg {
    opts: HashMap<String, String>,
}

impl Cfg {
    pub fn parse_from_file() -> Self {
        let home_dir = match std::env::var("HOME") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Couldn't find HOME from shell environment: {}", err);
                std::process::exit(1)
            }
        };
        let file_path = format!("{home_dir}/.config/scooby/config");

        let file = Config::builder()
            .add_source(config::File::with_name(&file_path))
            .build()
            .unwrap_or_default();

        let opts = match file.try_deserialize() {
            Ok(cfg) => cfg,
            Err(_) => {
                eprintln!("Couldn't deserialise config file!");
                std::process::exit(1)
            }
        };

        Cfg { opts }
    }

    /// this defaults to an empty string if no value is found from config file
    pub fn get(&self, key_name: &str) -> String {
        match self.opts.get(key_name) {
            Some(value) => value.to_string(),
            None => {
                eprintln!("No value associated with config key {}", key_name);
                String::new()
            }
        }
    }
}
