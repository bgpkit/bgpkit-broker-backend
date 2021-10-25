use crate::scrapers::*;
use log::info;
use futures::future::join_all;
use crate::kafka::KafkaProducer;

pub struct RouteViewsScraper{}

impl RouteViewsScraper {

    /// `scrape` implementation for RouteViews.
    ///
    /// Example of RouteViews2: http://archive.routeviews.org/bgpdata/
    pub async fn scrape(&self, collector: &Collector, latest: bool, db: Option<&DbConnection>, kafka: Option<&KafkaProducer>) -> Result<(), ScrapeError> {
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
                self.scrape_items(ribs_url, format!("{}-{}", month, "rib"), collector.id.clone(), rib_link_pattern.clone(), "rib".to_string(), db, kafka),
                self.scrape_items(updates_url, format!("{}-{}", month, "update"), collector.id.clone(), updates_link_pattern.clone(), "update".to_string(), db, kafka)
            ]
        }).collect();

        join_all(futures).await;
        Ok( () )
    }

    async fn scrape_items(&self, url: String, month: String, collector_id: String, pattern: Regex, data_type: String, db: Option<&DbConnection>, kafka: Option<&KafkaProducer>) -> Result<(), ScrapeError>{
        info!("scraping data for {} ... ", &month);
        let body = reqwest::get(&url).await?.text().await?;
        info!("     download for {} finished ", &month);

        let data_items: Vec<Item> =
        tokio::task::spawn_blocking(move || {
            pattern.captures_iter(body.as_str()).map(|cap| {
                // http://archive.routeviews.org/bgpdata/2001.11/UPDATES/updates.20011101.0923.bz2
                let url = format!("{}/{}", &url, cap[1].to_owned());
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.bz2.*"#).unwrap();
                let time_str = updates_link_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap().timestamp();
                Item {
                    url,
                    collector_id: collector_id.clone(),
                    timestamp: unix_time,
                    data_type: data_type.clone(),
                }
            }).collect()
        }).await.unwrap();

        if let Some(conn) = db {
            info!("   insert to db for {}...", &month);
            let inserted = conn.insert_items(&data_items);
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
        let _ = rv_scraper.scrape(&rv_collector, true, None, None).await;
    }
}
