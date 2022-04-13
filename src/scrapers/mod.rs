pub mod routeviews;
pub mod riperis;

use std::collections::HashMap;
use std::time::Duration;
use crate::models::*;
use crate::errors::*;
use regex::Regex;
use chrono::NaiveDateTime;
use log::warn;
use tokio::time::sleep;

pub use routeviews::RouteViewsScraper;
pub use riperis::RipeRisScraper;
use crate::db::DbConnection;
use futures::stream::StreamExt;

async fn verify_urls(urls: &Vec<String>) -> HashMap<String, i64> {
    // create stream of futures, 100 requests concurrent at most.
    let mut stream =
        futures::stream::iter(urls)
            .map( |url| verify_url_hyper(url.as_str()) )
            .buffer_unordered(100);

    let mut verified = HashMap::new();
    while let Some((url, size)) = stream.next().await {
        verified.insert(url, size);
    }

    return verified
}

async fn verify_url_hyper(url: &str) -> (String, i64) {
    let url_clone = url.to_string();

    let client = reqwest::Client::new();

    let mut retry_left = 3;
    let mut res = None;
    loop {
        res = match client.get(url_clone.as_str()).send().await{
            Ok(res) => {
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
        return (url_clone, 0)
    }
    let response = match res{
        None => {
            return (url_clone, 0)
        }
        Some(r) => {r}
    };

    if !response.status().is_success() {
        return (url_clone, 0)
    }
    let total_size = match response.content_length(){
        None => {
            0
        }
        Some(l) => {
            l as i64
        }
    };

    return if total_size > 0 {
        (url_clone, total_size)
    } else {
        (url_clone, total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verify() {
        let urls = verify_urls(
            &vec![
                "http://archive.routeviews.org/bgpdata/2022.02/RIBS/rib.20220221.0000.bz2".to_string(),
                "http://archive.routeviews.org/route-views2.saopaulo/bgpdata/2022.02/RIBS/rib.20220221.0000.bz2".to_string(),
                "https://data.ris.ripe.net/rrc01/2022.04/updates.20220402.1440.gz".to_string(),
                "http://archive.routeviews.org/route-views2.saopaulo/bgpdata/2022.02/RIBS/rib.20220221.0000.bz22".to_string(),
            ]
        ).await;
        dbg!(urls);
    }

    #[tokio::test]
    async fn test_verify_hyper() {
        let urls = verify_url_hyper(
                "http://archive.routeviews.org/bgpdata/2022.02/RIBS/rib.20220221.0000.bz2"
        ).await;
        dbg!(urls);
    }
}