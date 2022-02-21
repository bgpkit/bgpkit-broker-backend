pub mod schema;

use std::collections::HashSet;
use std::iter::FromIterator;
use chrono::NaiveDateTime;
use crate::models::*;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use log::{info, debug};

const CHUNK_SIZE: usize = 60_000;


pub struct DbConnection {
    conn: PgConnection,
}


impl DbConnection {
    pub fn new(db_url: &str) -> DbConnection {
        let conn = PgConnection::establish(db_url).unwrap();
        DbConnection{ conn }
    }

    pub fn insert_collectors(&self, entries: &Vec<Collector>){
        info!("inserting collectors info");
        use schema::collectors::dsl::*;
        diesel::insert_into(collectors)
            .values(entries)
            .on_conflict_do_nothing()
            .execute(&self.conn).unwrap();
    }

    pub fn get_urls_in_month(&self, collector: &str, month_str: &str) -> HashSet<String> {
        use schema::items::dsl::*;

        let start_ts = NaiveDateTime::parse_from_str(format!("{}.01T00:00:00", month_str).as_str(), "%Y.%m.%dT%H:%M:%S").unwrap();
        let end_ts = start_ts + chrono::Duration::days(31);
        HashSet::from_iter(items
            .filter(collector_id.eq(collector))
            .filter(timestamp.ge(start_ts.timestamp()))
            .filter(timestamp.le(end_ts.timestamp()))
            .select(url).load::<String>(&self.conn).unwrap().into_iter())
    }

    pub fn insert_items(&self, entries: &Vec<Item>) -> Vec<Item> {
        use schema::items::dsl::*;
        let chunks = entries.chunks(CHUNK_SIZE/4);
        let chunks_len = chunks.len();
        let mut inserted_items: Vec<Item> = vec![];
        for (i, chunk) in chunks.enumerate() {
            debug!("inserting {} chunk out of {} total chunks", i+1, chunks_len);
            inserted_items.extend(
                diesel::insert_into(items)
                .values(chunk)
                .on_conflict_do_nothing()
                .get_results(&self.conn).unwrap());
        }
        info!("tried to insert {} items, actually inserted {} items", entries.len(), inserted_items.len());
        return inserted_items;
    }

}

#[cfg(test)]
mod tests {
    use std::env;
    use super::*;

    #[test]
    fn test_insert() {
        env_logger::init();
        let conn = DbConnection::new("postgres://broker_user:broker@10.2.2.103/broker");

        let collectors = vec![Collector{
            id: "rrc00".to_string(),
            project: "riperis".to_string(),
            url: "http://data.ris.ripe.net/rrc00".to_string()
        }];
        let mut entries = vec![];
        info!("creating entries...");
        for t in 1..1_000_000 {
            entries.push(Item{
                collector_id: "rrc00".to_string(),
                timestamp: t,
                data_type: "rib".to_string(),
                url: "testurl".to_string()
            })
        };
        info!("creating entries... done");
        conn.insert_collectors(&collectors);
        conn.insert_items(&entries);
    }

    #[test]
    fn test_get_items() {
        let _ = dotenv::dotenv();
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let conn = DbConnection::new(db_url.as_str());
        let items = conn.get_urls_in_month("rrc25", "2022.02");
        for item in items {
            println!("{}", item)
        }
    }
}