use chrono::{Datelike, Utc};
use crate::scrapers::*;
use log::info;
use futures::StreamExt;
use tokio;
use crate::scrapers::utils::shift_months;

pub struct RipeRisScraper {
    pub mode: CrawlMode
}

impl RipeRisScraper {
    /// `scrape` implementation for RIPE RIS.
    pub async fn scrape(&self, collector: &Collector, db: Option<&DbConnection>) -> Result<(), ScrapeError> {
        info!("scraping RIPE RIS collector {}; only latest month = {}", collector.id, &self.mode);

        let months = match self.mode {
            CrawlMode::Latest => {
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
                        format!("{}.{:02}",ts2.year(), ts2.month()),
                        format!("{}.{:02}",ts.year(), ts.month())
                    ]
                }
            }
            CrawlMode::TwoMonths => {
                let ts = Utc::now();
                let ts2 = shift_months(ts, -1);
                vec![
                    format!("{}.{:02}",ts2.year(), ts2.month()),
                    format!("{}.{:02}",ts.year(), ts.month())
                ]
            }
            CrawlMode::Bootstrap => {
                let month_link_pattern: Regex = Regex::new(r#"<a href="(....\...)/">.*"#).unwrap();
                let body = reqwest::get(collector.url.as_str()).await?.text().await?;
                let mut res = vec![];
                for cap in month_link_pattern.captures_iter(body.as_str()) {
                    let month = cap[1].to_owned();
                    if let Some(conn) = db {
                        if conn.count_records_in_month(collector.id.as_str(), month.as_str()).await > 0 {
                            info!("skip month {} for {} in bootstrap mode", month.as_str(), collector.id.as_str());
                            continue
                        }
                    }
                    res.push(month)
                }
                res
            }
        };

        info!("total of {} months to scrape", months.len());

        let mut stream = futures::stream::iter(months.clone()).map(|month| {
            let url = format!("{}/{}", collector.url, month);
            self.scrape_month(
                url,
                month,
                collector.id.clone(),
                db,
            )
        }).buffer_unordered(100);
        while let Some(_res) = stream.next().await {
        }

        Ok( () )
    }

    async fn scrape_month(&self, url: String, month: String, collector_id: String, db: Option<&DbConnection>) -> Result<(), ScrapeError>{
        info!("scraping data for {} {} ...", collector_id.as_str(), &month);
        let body = reqwest::get(url.clone()).await?.text().await?;
        info!("    download for {} {} finished ", collector_id.as_str(), &month);

        let collector_clone = collector_id.clone();

        let data_items: Vec<Item> =
        tokio::task::spawn_blocking(move || {
            let items = extract_link_size(body.as_str());
            items.iter().map(|(link, size)|{
                let url = format!("{}/{}",url, link).replace("http", "https");
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.gz.*"#).unwrap();
                let time_str = updates_link_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap();
                match link.contains("update") {
                    true => Item {
                        ts_start: unix_time,
                        ts_end: unix_time + chrono::Duration::seconds(5*60),
                        url: url.clone(),
                        rough_size: size.clone(),
                        exact_size: 0,
                        collector_id: collector_id.clone(),
                        data_type: "update".to_string(),
                    },
                    false => Item {
                        ts_start: unix_time,
                        ts_end: unix_time,
                        url: url.clone(),
                        rough_size: size.clone(),
                        exact_size: 0,
                        collector_id: collector_id.clone(),
                        data_type: "rib".to_string(),
                    }
                }
            }).collect()
        }).await.unwrap();

        if let Some(conn) = db {
            info!("    insert to db for {} {}...", collector_clone.as_str(), &month);

            let to_insert = match self.mode {
                CrawlMode::Latest | CrawlMode::TwoMonths => {
                    let current_month_items = conn.get_urls_in_month(collector_clone.as_str(), month.as_str()).await;
                    data_items.into_iter().filter(|x|!current_month_items.contains(&x.url))
                        .collect::<Vec<Item>>()
                }
                CrawlMode::Bootstrap => {
                    data_items
                }
            };

            let inserted = conn.insert_items(&to_insert).await;

            #[cfg(feature = "kafka")]
            conn.notify(&inserted).await;

            info!("    insert to db for {} {}... {}/{} inserted", collector_clone.as_str(), &month, to_insert.len(), inserted.len());
        }

        info!("scraping data for {} ... finished", &month);
        Ok(())
    }
}