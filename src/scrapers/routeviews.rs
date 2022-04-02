use crate::scrapers::*;
use log::{info, warn};
use futures::future::join_all;
use crate::kafka::KafkaProducer;

pub struct RouteViewsScraper{}

impl RouteViewsScraper {

    /// `scrape` implementation for RouteViews.
    ///
    /// Example of RouteViews2: http://archive.routeviews.org/bgpdata/
    pub async fn scrape(&self, collector: &Collector, latest: bool, db: Option<&DbConnection>, kafka: Option<&KafkaProducer>, verify: bool) -> Result<(), ScrapeError> {
        info!("scraping RouteViews collector {}; only latest month = {}", collector.id, &latest);

        let month_link_pattern: Regex = Regex::new(r#"<a href="(....\...)/">.*"#).unwrap();
        let rib_link_pattern: Regex = Regex::new(r#"<a href="(rib\..*\.bz2)">.*"#).unwrap();
        let updates_link_pattern: Regex = Regex::new(r#"<a href="(updates\..*\.bz2)">.*"#).unwrap();

        let body = reqwest::get(collector.url.as_str()).await?.text().await?;
        let mut months: Vec<String> = month_link_pattern.captures_iter(body.as_str()).map(|cap|{
            cap[1].to_owned()
        }).collect();

        if latest {
            // take the latest 2 months for scraping.
            months = months.into_iter().rev().take(2).collect();
        }

        info!("total of {} months to scrape", months.len());

        let futures: Vec<_> = months.iter().flat_map(|month| {
            let ribs_url = format!("{}/{}/RIBS", collector.url, month);
            let updates_url = format!("{}/{}/UPDATES", collector.url, month);

            [
                self.scrape_items(ribs_url, month.to_string(), "rib".to_string(), collector.id.clone(), rib_link_pattern.clone(), "rib".to_string(), db, kafka, verify),
                self.scrape_items(updates_url, month.to_string(), "update".to_string(), collector.id.clone(), updates_link_pattern.clone(), "update".to_string(), db, kafka, verify)
            ]
        }).collect();

        join_all(futures).await;
        Ok( () )
    }

    async fn scrape_items(&self, url: String, month: String, data_type_str: String, collector_id: String, pattern: Regex, data_type: String, db: Option<&DbConnection>, kafka: Option<&KafkaProducer>, verify: bool) -> Result<(), ScrapeError>{
        info!("scraping data for {}-{} ... ", &month, &data_type_str);
        let body = reqwest::get(&url).await?.text().await?;
        info!("     download for {}-{} finished ", &month, &data_type_str);

        let collector_clone = collector_id.clone();

        let data_items: Vec<Item> =
        tokio::task::spawn_blocking(move || {
            pattern.captures_iter(body.as_str()).map(|cap| {
                // http://archive.routeviews.org/bgpdata/2001.11/UPDATES/updates.20011101.0923.bz2
                let url = format!("{}/{}", &url, cap[1].to_owned());
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.bz2.*"#).unwrap();
                let time_str = updates_link_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap().timestamp();
                let interval = match data_type.as_str(){
                    "rib" => 0,
                    "update" => 15*60,
                    _ => 0
                };
                Item {
                    ts_start: unix_time,
                    ts_end: unix_time+interval,
                    file_size: 0,
                    collector_id: collector_id.clone(),
                    data_type: data_type.clone(),
                    file_info: Default::default(),
                    url,
                }
            }).collect()
        }).await.unwrap();

        if let Some(conn) = db {
            info!("   insert to db for {}-{}...", &month, &data_type_str);
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
                                file_info: x.file_info,
                                url: x.url,
                            }
                        )
                    } else {
                        warn!("    file {} is empty", &x.url);
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
    use super::*;


    #[tokio::test]
    async fn test_routeviews() {
        let rv_collector = Collector{
            id: "rv2".to_string(),
            project: "routeviews".to_string(),
            url: "http://archive.routeviews.org/bgpdata".to_string()
        };
        let rv_scraper = RouteViewsScraper{};
        let _ = rv_scraper.scrape(&rv_collector, true, None, None, false).await;
    }
}
