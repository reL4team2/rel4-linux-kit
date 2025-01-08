use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    services: Vec<Service>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Service {
    name: String,
    file: String,
    mmio: Option<Vec<(usize, usize, usize)>>,
}

fn main() {
    let content = fs::read_to_string("services.toml").unwrap();
    let service: Config = toml::from_str(&content).unwrap();
}
