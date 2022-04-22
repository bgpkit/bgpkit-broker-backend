pub mod routeviews;
pub mod riperis;

use std::time::Duration;
use crate::db::models::*;
use crate::errors::*;
use regex::Regex;
use chrono::NaiveDateTime;
use log::{info, warn};
use scraper::{Html, Selector};
use tokio::time::sleep;

pub use routeviews::RouteViewsScraper;
pub use riperis::RipeRisScraper;
use crate::db::DbConnection;

const SIZE_KB: u64 = u64::pow(1024,1);
const SIZE_MB: u64 = u64::pow(1024,2);
const SIZE_GB: u64 = u64::pow(1024,3);

pub async fn check_size(mut item: Item) -> Option<Item> {
    let url_clone = item.url.to_string();

    let client = reqwest::Client::new();

    info!("checking {}", url_clone.as_str());
    let mut retry_left = 3;
    let mut res = None;
    loop {
        res = match client.get(url_clone.as_str()).send().await{
            Ok(res) => {
                info!("finished checking {}", url_clone.as_str());
                Some(res)
            }
            Err(e) => {
                if retry_left == 0 {
                    warn!("give up retry {}", url_clone.as_str());
                    break
                }
                retry_left -= 1;
                warn!("error: {}; retry downloading {}", e.to_string(), url_clone.as_str());
                sleep(Duration::from_millis(1000)).await;
                continue
            }
        };
        break
    }

    if res.is_none() {
        return None
    }

    let response = match res{
        None => {
            return None
        }
        Some(r) => {r}
    };

    if !response.status().is_success() {
        return None
    }
    let total_size = match response.content_length(){
        None => {
            return None
        }
        Some(l) => {
            l as i64
        }
    };

    item.exact_size = total_size;

    return Some(item)
}


fn size_str_to_bytes(size_str: &str, size_pattern: &Regex) -> i64 {
    let cap = size_pattern.captures(size_str).unwrap();
    let mut size = cap[1].to_string().parse::<f64>().unwrap();
    size *= match cap[2].to_ascii_lowercase().as_str() {
        "k" => SIZE_KB,
        "m" => SIZE_MB,
        "g" => SIZE_GB,
        "" => 1,
        other => panic!("unknown file size multiplier {}", other)
    } as f64;
    size as i64
}

pub fn extract_link_size(body: &str) -> Vec<(String, i64)>{
    let size_pattern: Regex = Regex::new(r#" *([\d.]+)([MKGmkg]*)"#).unwrap();
    let mut res: Vec<(String, i64)> = vec![];

    let fragment = Html::parse_fragment(body);
    let row_selector = Selector::parse("tr").unwrap();
    let link_selector = Selector::parse("a").unwrap();
    for elem in  fragment.select(&row_selector) {
        let text_arr = elem.text().filter(|t| t.is_ascii() && !t.trim().is_empty()).collect::<Vec<_>>();
        let text = text_arr.join("");
        if text.is_empty() || text.contains("Name") || text.contains("Parent") {
            continue
        }
        let href = elem.select(&link_selector).next().unwrap().value().attr("href");
        res.push((href.unwrap().to_string(), size_str_to_bytes(text_arr[2], &size_pattern)));
    }
    res
}