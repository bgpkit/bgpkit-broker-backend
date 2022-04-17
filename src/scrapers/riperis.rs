use crate::scrapers::*;
use log::info;
use futures::StreamExt;
use tokio;

pub struct RipeRisScraper {
    pub update_mode: bool
}

impl RipeRisScraper {
    /// `scrape` implementation for RIPE RIS.
    pub async fn scrape(&self, collector: &Collector, latest: bool, db: Option<&DbConnection>) -> Result<(), ScrapeError> {
        info!("scraping RIPE RIS collector {}; only latest month = {}", collector.id, &latest);

        let month_link_pattern: Regex = Regex::new(r#"<a href="(....\...)/">.*"#).unwrap();
        let rib_link_pattern: Regex = Regex::new(r#"<a href="(b?view\..*\.gz)">.*"#).unwrap();
        let updates_link_pattern: Regex = Regex::new(r#"<a href="(updates\..*\.gz)">.*"#).unwrap();

        let body = reqwest::get(collector.url.as_str()).await?.text().await?;
        let mut months: Vec<String> = month_link_pattern.captures_iter(body.as_str()).filter_map(|cap|{
            let month = cap[1].to_owned();
            if !latest {
                if let Some(conn) = db {
                    if conn.count_records_in_month(collector.id.as_str(), month.as_str()) > 0 {
                        info!("skip month {} for {} in bootstrap mode", month.as_str(), collector.id.as_str());
                        return None
                    }
                }
            }
            Some(month)
        }).collect();

        if latest {
            // take the latest 2 months for scraping.
            months = months.into_iter().take(2).collect();
        }

        info!("total of {} months to scrape", months.len());

        let mut stream = futures::stream::iter(months.clone()).map(|month| {
            let url = format!("{}/{}", collector.url, month);
            self.scrape_month(
                url,
                month,
                updates_link_pattern.clone(),
                rib_link_pattern.clone(),
                collector.id.clone(),
                db,
            )
        }).buffer_unordered(100);
        while let Some(_res) = stream.next().await {
        }

        Ok( () )
    }

    async fn scrape_month(&self, url: String, month: String, update_pattern: Regex, rib_pattern: Regex, collector_id: String, db: Option<&DbConnection>) -> Result<(), ScrapeError>{
        info!("scraping data for {} {} ...", collector_id.as_str(), &month);
        let body = reqwest::get(url.clone()).await?.text().await?;
        info!("    download   for {} finished", &month);

        let collector_clone = collector_id.clone();

        let data_items =
        tokio::task::spawn_blocking(move || {
            let mut data_items = vec![];
            update_pattern.captures_iter(body.as_str()).for_each(|cap|{
                let url = format!("{}/{}",url, cap[1].to_owned()).replace("http", "https");
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.gz.*"#).unwrap();
                let time_str = updates_link_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap();
                data_items.push(
                    Item {
                        ts_start: unix_time,
                        ts_end: unix_time + chrono::Duration::seconds(5*60),
                        url: url.clone(),
                        file_size: 0,
                        collector_id: collector_id.clone(),
                        data_type: "update".to_string(),
                    }
                );
            });

            rib_pattern.captures_iter(body.as_str()).for_each(|cap|{
                let url = format!("{}/{}",url, cap[1].to_owned()).replace("http", "https");
                let url_time_pattern: Regex = Regex::new(r#".*(........\.....)\.gz.*"#).unwrap();
                let time_str = url_time_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap();
                data_items.push(
                    Item {
                        ts_start: unix_time,
                        ts_end: unix_time,
                        url: url.clone(),
                        file_size: 0,
                        collector_id: collector_id.clone(),
                        data_type: "rib".to_string(),
                    });
            });
            data_items
        }).await.unwrap();

        if let Some(conn) = db {
            info!("    insert to db for {} {}...", collector_clone.as_str(), &month);

            let to_insert = if self.update_mode {
                let current_month_items = conn.get_urls_in_month(collector_clone.as_str(), month.as_str());
                data_items.into_iter().filter(|x|!current_month_items.contains(&x.url))
                    .collect::<Vec<Item>>()
            } else {
                data_items
            };

            let inserted = conn.insert_items(&to_insert);
            info!("    insert to db for {} {}... {}/{} inserted", collector_clone.as_str(), &month, to_insert.len(), inserted.len());
        }

        info!("scraping data for {} ... finished", &month);
        Ok(())
    }
}