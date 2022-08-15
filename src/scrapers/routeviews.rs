use chrono::{Datelike, Utc};
use crate::scrapers::*;
use log::info;
use futures::StreamExt;

pub struct RouteViewsScraper{
    pub update_mode: bool,
}

impl RouteViewsScraper {

    /// `scrape` implementation for RouteViews.
    ///
    /// Example of RouteViews2: http://archive.routeviews.org/bgpdata/
    pub async fn scrape(&self, collector: &Collector, latest: bool, db: Option<&DbConnection>) -> Result<(), ScrapeError> {
        info!("scraping RouteViews collector {}; only latest month = {}", collector.id, &latest);

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
            let mut res = vec![];
            for cap in month_link_pattern.captures_iter(body.as_str()) {
                let month = cap[1].to_owned();
                if !latest {
                    if let Some(conn) = db {
                        if conn.count_records_in_month(collector.id.as_str(), month.as_str()).await > 0 {
                            info!("skip month {} for {} in bootstrap mode", month.as_str(), collector.id.as_str());
                            continue
                        }
                    }
                }
                res.push(month)
            }
            res
        };

        info!("total of {} months to scrape", months.len());

        let mut stream = futures::stream::iter(months.clone()).map(|month| {
            let ribs_url = format!("{}/{}/RIBS", collector.url, month);
            self.scrape_items(ribs_url, month, "rib".to_string(), collector.id.clone(), db)
        }).buffer_unordered(100);
        while let Some(_res) = stream.next().await { }

        let mut stream = futures::stream::iter(months).map(|month| {
            let updates_url = format!("{}/{}/UPDATES", collector.url, month);
            self.scrape_items(updates_url, month, "update".to_string(), collector.id.clone(), db)
        }).buffer_unordered(100);
        while let Some(_res) = stream.next().await {  }

        Ok( () )
    }

    async fn scrape_items(&self, url: String, month: String, data_type_str: String, collector_id: String, db: Option<&DbConnection>) -> Result<(), ScrapeError>{
        info!("scraping data for {} {}-{} ... ", collector_id.as_str(), &month, &data_type_str);
        let body = reqwest::get(url.clone()).await?.text().await?;
        info!("    download for {} {}-{} finished ", collector_id.as_str(), &month, &data_type_str);

        let collector_clone = collector_id.clone();

        let data_items: Vec<Item> =
        tokio::task::spawn_blocking(move || {
            let items = extract_link_size(body.as_str());
            items.iter().map(|(link, size)| {
                // http://archive.routeviews.org/bgpdata/2001.11/UPDATES/updates.20011101.0923.bz2
                let url = format!("{}/{}", &url, link);
                let updates_link_pattern: Regex = Regex::new(r#".*(........\.....)\.bz2.*"#).unwrap();
                let time_str = updates_link_pattern.captures(&url).unwrap().get(1).unwrap().as_str();
                let unix_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d.%H%M").unwrap();
                let interval = match data_type_str.as_str(){
                    "rib" => chrono::Duration::seconds(0),
                    "update" => chrono::Duration::seconds(15*60),
                    _ => panic!("unknown data type {}", data_type_str.as_str())
                };

                Item {
                    ts_start: unix_time,
                    ts_end: unix_time+interval,
                    rough_size: *size,
                    exact_size: 0,
                    collector_id: collector_id.clone(),
                    data_type: data_type_str.clone(),
                    url,
                }
            }).collect()
        }).await.unwrap();

        if let Some(conn) = db {
            info!("    insert to db for {} {}...", collector_clone.as_str(), &month);

            let to_insert = if self.update_mode {
                let current_month_items = conn.get_urls_in_month(collector_clone.as_str(), month.as_str()).await;
                data_items.into_iter().filter(|x|!current_month_items.contains(&x.url))
                    .collect::<Vec<Item>>()
            } else {
                data_items
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

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_routeviews() {
        env_logger::init();

        let _rv_collector = Collector{
            id: "rv2".to_string(),
            project: "routeviews".to_string(),
            url: "http://archive.routeviews.org/bgpdata".to_string()
        };
        let rv_scraper = RouteViewsScraper{ update_mode: true };
        // let _ = rv_scraper.scrape(&rv_collector, true, None).await;
        rv_scraper.scrape_items(
            "http://archive.routeviews.org/route-views.linx/bgpdata/2014.03/RIBS".to_string(),
            "2004.03".to_string(),
            "rib".to_string(),
            "route-views.linx".to_string(),
            None
        ).await.unwrap();
    }

}
