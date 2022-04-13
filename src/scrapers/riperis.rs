use crate::scrapers::*;
use log::info;
use futures::future::join_all;
use tokio;
use crate::kafka::KafkaProducer;

pub struct RipeRisScraper {}

impl RipeRisScraper {
    /// `scrape` implementation for RIPE RIS.
    pub async fn scrape(&self, collector: &Collector, latest: bool, db: Option<&DbConnection>, kafka: Option<&KafkaProducer>, verify: bool) -> Result<(), ScrapeError> {
        info!("scraping RIPE RIS collector {}; only latest month = {}", collector.id, &latest);

        let month_link_pattern: Regex = Regex::new(r#"<a href="(....\...)/">.*"#).unwrap();
        let rib_link_pattern: Regex = Regex::new(r#"<a href="(b?view\..*\.gz)">.*"#).unwrap();
        let updates_link_pattern: Regex = Regex::new(r#"<a href="(updates\..*\.gz)">.*"#).unwrap();

        let body = reqwest::get(collector.url.as_str()).await?.text().await?;
        let mut months: Vec<String> = month_link_pattern.captures_iter(body.as_str()).map(|cap|{
            cap[1].to_owned()
        }).collect();

        if latest {
            // take the latest 2 months for scraping.
            months = months.into_iter().take(2).collect();
        }

        info!("total of {} months to scrape", months.len());

        let mut futures = vec![];
        for month in months {
            let url = format!("{}/{}", collector.url, month);
            futures.push(self.scrape_month(
                url,
                month.clone(),
                updates_link_pattern.clone(),
                rib_link_pattern.clone(),
                collector.id.clone(),
                db,
                kafka,
                verify
            ))
        }

        join_all(futures).await;
        Ok( () )
    }

    async fn scrape_month(&self, url: String, month: String, update_pattern: Regex, rib_pattern: Regex, collector_id: String, db: Option<&DbConnection>, kafka: Option<&KafkaProducer>, verify: bool) -> Result<(), ScrapeError>{
        info!("scraping data for {} ...", &month);
        let body = reqwest::get(url.clone()).await?.text().await?;
        info!("   download   for {} finished", &month);

        let collector_clone = collector_id.clone();

        let data_items =
        tokio::task::spawn_blocking(move || {
            let mut data_items = vec![];
            update_pattern.captures_iter(body.as_str()).for_each(|cap|{
                let url = format!("{}/{}",url, cap[1].to_owned()).replace("http", "https");
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.gz.*"#).unwrap();
                let time_str = updates_link_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap().timestamp();
                data_items.push(
                    Item {
                        ts_start: unix_time,
                        ts_end: unix_time + 5*60,
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
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap().timestamp();
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
            info!("   insert to db for {}...", &month);
            let new_items = if verify{
                let current_month_items = conn.get_urls_in_month(collector_clone.as_str(), month.as_str());
                let new_urls = data_items.iter().filter(|x| !current_month_items.contains(&x.url))
                    .map(|x| x.url.clone())
                    .collect::<Vec<String>>();
                let file_sizes = verify_urls(&new_urls).await;
                let good_count = file_sizes.values().filter(|x| **x>0).count();
                info!("    total {} new urls, {} verified working", new_urls.len(), good_count);
                data_items.into_iter().filter_map(|x| {
                    if current_month_items.contains(&x.url) {
                        return None
                    }
                    let file_size = file_sizes.get(&x.url).unwrap().to_owned();
                    if file_size>0 {
                        Some(
                            Item {
                                ts_start: x.ts_start,
                                ts_end: x.ts_end,
                                file_size: file_size as i64,
                                collector_id: x.collector_id,
                                data_type: x.data_type,
                                url: x.url,
                            }
                        )
                    } else {
                        None
                    }
                }
                )
                    .collect::<Vec<Item>>()
            } else {
                data_items
            };
            let inserted = conn.insert_items(&new_items);
            if let Some(producer) = kafka {
                if inserted.len()>0 {
                    info!("   announcing new items to kafka ...");
                    producer.produce(&inserted).await;
                }
            }
        }

        info!("scraping data for {} ... finished", &month);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::*;
    use env_logger;

    #[tokio::test]
    async fn test_routeviews() {
    }

    #[tokio::test]
    async fn test_routeviews_with_db() {
        env_logger::init();
        let _ = dotenv::dotenv();
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let conn = DbConnection::new(db_url.as_str());

        let ris_collector = Collector{
            id: "rrc00".to_string(),
            project: "riperis".to_string(),
            url: "http://data.ris.ripe.net/rrc00".to_string()
        };
        let ris_scraper = RipeRisScraper{};
        ris_scraper.scrape(&ris_collector, true, Some(&conn), None, true).await.unwrap();
    }

}
