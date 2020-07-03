//! This module deals with Town of Salem fandom (formerly wikia) API.

use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://town-of-salem.fandom.com/api/v1/";

#[derive(Debug, Deserialize)]
pub(crate) struct Resp {
    pub(crate) items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Item {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) url: String,
}

pub(crate) async fn get_items(client: &Client, input: &str) -> Option<Vec<Item>> {
    let req_builder =
        client.get(format!("{}/Search/List?query={}&limit=5", BASE_URL, input).as_str());
    let res = match req_builder.send().await {
        Ok(res) => res,
        Err(_) => return None,
    };

    match res.json::<Resp>().await {
        Ok(v) => {
            let mut res = Vec::new();
            for item in v.items {
                if item
                    .title
                    .to_ascii_lowercase()
                    .contains(&input.to_ascii_lowercase())
                {
                    res.push(item);
                }
            }
            Some(res)
        }
        Err(_) => None,
    }
}
