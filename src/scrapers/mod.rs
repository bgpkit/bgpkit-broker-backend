pub mod routeviews;
pub mod riperis;

use std::time::Duration;
use crate::models::*;
use crate::errors::*;
use regex::Regex;
use chrono::NaiveDateTime;
use log::{info, warn};
use tokio::time::sleep;

pub use routeviews::RouteViewsScraper;
pub use riperis::RipeRisScraper;
use crate::db::DbConnection;

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

    item.file_size = total_size;

    return Some(item)
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