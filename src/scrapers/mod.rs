pub mod routeviews;
pub mod riperis;
mod utils;

use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::db::models::*;
use crate::errors::*;
use regex::Regex;
use chrono::NaiveDateTime;
use scraper::{Html, Selector};

pub use routeviews::RouteViewsScraper;
pub use riperis::RipeRisScraper;
use crate::db::DbConnection;

const SIZE_KB: u64 = u64::pow(1024,1);
const SIZE_MB: u64 = u64::pow(1024,2);
const SIZE_GB: u64 = u64::pow(1024,3);

#[derive(Debug, Clone, Copy)]
pub enum CrawlMode {
    Latest,
    Bootstrap,
    TwoMonths
}

impl Display for CrawlMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self{
            CrawlMode::Latest => {write!(f, "latest")}
            CrawlMode::Bootstrap => {write!(f, "bootstrap")}
            CrawlMode::TwoMonths => {write!(f, "two_months")}
        }
    }
}

impl FromStr for CrawlMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "latest" => Ok(CrawlMode::Latest),
            "two_months" => Ok(CrawlMode::TwoMonths),
            "bootstrap" => Ok(CrawlMode::Bootstrap),
            _ => Err("crawl mode must be one of the: ['latest', 'two_months', 'bootstrap']".to_string())
        }
    }
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