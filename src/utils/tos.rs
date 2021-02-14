//! This module deals with Town of Salem fandom (formerly wikia) API.

use reqwest::Client;
use serde::Deserialize;

const TOS_BASE: &str = "https://town-of-salem.fandom.com/wiki";
const TOS_API_BASE: &str = "https://town-of-salem.fandom.com/api.php";

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(rename = "query")]
    search: Search,
}

#[derive(Debug, Deserialize)]
struct Search {
    #[serde(rename = "search")]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    title: String,
}

pub(crate) struct SearchResult {
    pub(crate) title: String,
    pub(crate) url: String,
}

pub(crate) async fn search(client: &Client, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Sync + Send>> {
    let request_builder = client.get(TOS_API_BASE).query(&[
        ("action", "query"),
        ("list", "search"),
        ("srsearch", query),
        ("srlimit", "5"),
        ("format", "json"),
        ("srprop", ""),
    ]);

    let response = request_builder.send().await?.json::<Resp>().await?;

    Ok(response.search.items.into_iter().filter_map(|i| {
        if i.title.to_ascii_lowercase().contains(&query.to_ascii_lowercase()) {
            let url = format!("{}/{}", TOS_BASE, &i.title.replace(" ", "_"));
            Some(SearchResult {
                url,
                title: i.title,
            })
        } else {
            None
        }
    })
    .collect())
}
