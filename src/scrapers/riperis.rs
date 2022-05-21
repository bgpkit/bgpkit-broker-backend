use chrono::{Datelike, Utc};
use crate::scrapers::*;
use log::{info, warn};
use futures::StreamExt;
use tokio;
use crate::db::*;

pub struct RipeRisScraper {
    pub update_mode: bool
}

impl RipeRisScraper {
    /// `scrape` implementation for RIPE RIS.
    pub async fn scrape(&self, collector: &Collector, latest: bool, db_path: &str) -> Result<(), ScrapeError> {
        info!("scraping RIPE RIS collector {}; only latest month = {}", collector.id, &latest);

        let months = if latest {
            let ts = Utc::now();
            let ts2 = ts - chrono::Duration::days(1);
            if ts.month() == ts2.month() {
                vec![
                    format!("{}.{:02}",ts.year(), ts.month())
                ]
            } else {
                // on borderline date, i.e. on the end of a month
                // we check both current and previous month to make sure we don't miss anything
                vec![
                    format!("{}.{:02}",ts.year(), ts2.month()),
                    format!("{}.{:02}",ts.year(), ts.month())
                ]
            }
        } else {
            let month_link_pattern: Regex = Regex::new(r#"<a href="(....\...)/">.*"#).unwrap();
            let body = reqwest::get(collector.url.as_str()).await?.text().await?;
            let db = match db_path {
                "" => None,
                p => Some(BrokerDb::new(p))
            };
            month_link_pattern.captures_iter(body.as_str()).filter_map(|cap|{
                let month = cap[1].to_owned();
                if !latest {
                    if let Some(conn) = &db {
                        if conn.count_records_in_month(collector.id.as_str(), month.as_str()) > 0 {
                            info!("skip month {} for {} in bootstrap mode", month.as_str(), collector.id.as_str());
                            return None
                        }
                    }
                }
                Some(month)
            }).collect()
        };

        info!("total of {} months to scrape", months.len());

        let mut stream = futures::stream::iter(months.clone()).map(|month| {
            let url = format!("{}/{}", collector.url, month);
            self.scrape_month(
                url,
                month,
                collector.id.clone(),
                db_path,
            )
        }).buffer_unordered(50);
        while let Some(_res) = stream.next().await {
        }

        Ok( () )
    }

    async fn scrape_month(&self, url: String, month: String, collector_id: String, db_path: &str) -> Result<(), ScrapeError>{
        let db = match db_path {
            "" => None,
            p => Some(BrokerDb::new(p))
        };
        info!("scraping data for {} {} ...", collector_id.as_str(), &month);
        let body = reqwest::get(url.clone()).await?.text().await?;
        info!("    download for {} {} finished ", collector_id.as_str(), &month);

        let collector_clone = collector_id.clone();

        let data_items: Vec<Item> =
        tokio::task::spawn_blocking(move || {
            let items = extract_link_size(body.as_str());
            items.iter().filter_map(|(link, size)|{
                let url = format!("{}/{}",url, link).replace("http", "https");
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.gz.*"#).unwrap();
                let time_str = match updates_link_pattern.captures(&url) {
                    Some(p) => {
                        match p.get(1){
                            Some(s) => s.as_str(),
                            None => {
                                warn!("unrecognized link: {}", url);
                                return None
                            }
                        }
                    },
                    None => {
                        warn!("unrecognized link: {}", url);
                        return None
                    }
                };
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap();
                match link.contains("update") {
                    true => Some(Item {
                        ts_start: unix_time,
                        ts_end: unix_time + chrono::Duration::seconds(5*60-1),
                        url: url.clone(),
                        rough_size: *size,
                        exact_size: 0,
                        collector_id: collector_id.clone(),
                        data_type: "update".to_string(),
                    }),
                    false => Some(Item {
                        ts_start: unix_time,
                        ts_end: unix_time,
                        url: url.clone(),
                        rough_size: *size,
                        exact_size: 0,
                        collector_id: collector_id.clone(),
                        data_type: "rib".to_string(),
                    })
                }
            }).collect()
        }).await.unwrap();

        if let Some(mut db) = db {
            info!("    insert to db for {} {}...", collector_clone.as_str(), &month);
            let inserted = db.insert_items(&data_items);
            info!("    insert to db for {} {}... {}/{} inserted", collector_clone.as_str(), &month, data_items.len(), inserted);
        }

        info!("scraping data for {} ... finished", &month);
        Ok(())
    }
}