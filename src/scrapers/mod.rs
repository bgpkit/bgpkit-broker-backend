pub mod routeviews;
pub mod riperis;

use std::collections::HashMap;
use crate::models::*;
use crate::errors::*;
use regex::Regex;
use chrono::NaiveDateTime;
use futures::future::join_all;

pub use routeviews::RouteViewsScraper;
pub use riperis::RipeRisScraper;
use crate::db::DbConnection;

async fn verify_urls(urls: &Vec<String>) -> HashMap<String, u64> {
    let mut futures = vec![];
    for url in urls{
        futures.push(verify_url(url.as_str())
        );
    }
    let res = join_all(futures).await;
    let mut verified = HashMap::new();
    for (url, size) in res {
        verified.insert(url, size);
    }

    return verified
}

async fn verify_url(url: &str) -> (String, u64) {
    let url_clone = url.to_string();
    let res = match reqwest::Client::new()
        .get(url)
        .send()
        .await
        .or(Err(format!("{}", &url))) {
        Ok(r) => {r}
        Err(_) => {return (url_clone, 0)}
    };
    if !res.status().is_success() {
        return (url_clone, 0)
    }
    let total_size = match res
        .content_length(){
        None => {0}
        Some(l) => {l}
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
                "http://archive.routeviews.org/route-views2.saopaulo/bgpdata/2022.02/RIBS/rib.20220221.0000.bz22".to_string(),
            ]
        ).await;
        dbg!(urls);
    }
}