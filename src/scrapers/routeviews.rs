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

        let mut stream = futures::stream::iter(months.clone()).map(|month| {
            let ribs_url = format!("{}/{}/RIBS", collector.url, month);
            self.scrape_items(ribs_url, latest, month.to_string(), "rib".to_string(), collector.id.clone(), rib_link_pattern.clone(), "rib".to_string(), db)
        }).buffer_unordered(100);
        while let Some(res) = stream.next().await { res.unwrap() }

        let mut stream = futures::stream::iter(months).map(|month| {
            let updates_url = format!("{}/{}/UPDATES", collector.url, month);
            self.scrape_items(updates_url, latest, month.to_string(), "update".to_string(), collector.id.clone(), updates_link_pattern.clone(), "update".to_string(), db)
        }).buffer_unordered(100);
        while let Some(res) = stream.next().await { res.unwrap() }

        Ok( () )
    }

    async fn scrape_items(&self, url: String, latest: bool, month: String, data_type_str: String, collector_id: String, pattern: Regex, data_type: String, db: Option<&DbConnection>) -> Result<(), ScrapeError>{
        info!("scraping data for {} {}-{} ... ", collector_id.as_str(), &month, &data_type_str);
        let body = reqwest::get(&url).await?.text().await?;
        info!("     download for {} {}-{} finished ", collector_id.as_str(), &month, &data_type_str);

        if !latest {
            if let Some(conn) = db {
                let current_month_items = conn.get_urls_in_month(collector_id.as_str(), month.as_str());
                if !current_month_items.is_empty() {
                    info!("skip month {} for {} in bootstrap mode", month.as_str(), collector_id.as_str());
                    return Ok(())
                }
            }
        }

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
                    url,
                }
            }).collect()
        }).await.unwrap();

        if let Some(conn) = db {
            info!("   insert to db for {} {}...", collector_clone.as_str(), &month);

            let to_insert = if self.update_mode {

                let current_month_items = conn.get_urls_in_month(collector_clone.as_str(), month.as_str());
                data_items.into_iter().filter(|x|!current_month_items.contains(&x.url))
                    .collect::<Vec<Item>>()
            } else {
                data_items
            };

            let inserted = conn.insert_items(&to_insert);
            info!("tried to insert {} items, actually inserted {} items", to_insert.len(), inserted.len());
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
        let rv_scraper = RouteViewsScraper{ update_mode: true };
        let _ = rv_scraper.scrape(&rv_collector, true, None).await;
    }
}
