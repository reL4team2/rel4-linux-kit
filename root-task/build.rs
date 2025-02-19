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
    dma: Option<Vec<(usize, usize)>>,
}

fn main() {
    let content = fs::read_to_string("services.toml").unwrap();
    let config: Config = toml::from_str(&content).unwrap();
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .unwrap()
        .parse(include_str!("template.rs.liquid"))
        .unwrap();
    // template.render(liquid::object! {
    //     services: config
    // }).unwrap();
}
