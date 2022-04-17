pub mod schema;

use std::collections::HashSet;
use std::iter::FromIterator;
use chrono::NaiveDateTime;
use diesel::dsl::count;
use crate::models::*;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use log::{info, debug};

const CHUNK_SIZE: usize = 60_000;


pub struct DbConnection {
    pub conn: PgConnection,
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

    pub fn count_records_in_month(&self, collector: &str, month_str: &str) -> i64 {
        use schema::items::dsl::*;

        let start_ts = match NaiveDateTime::parse_from_str(format!("{}.01T00:00:00", month_str).as_str(), "%Y.%m.%dT%H:%M:%S"){
            Ok(t) => {t}
            Err(e) => {
                panic!("parsing {} failed: {}", month_str, e.to_string())
            }
        };
        let end_ts = start_ts + chrono::Duration::days(31);
        items
            .filter(collector_id.eq(collector))
            .filter(ts_start.ge(start_ts))
            .filter(ts_start.le(end_ts))
            .select(count(url)).first::<i64>(&self.conn).unwrap()
    }

    pub fn get_urls_in_month(&self, collector: &str, month_str: &str) -> HashSet<String> {
        use schema::items::dsl::*;

        let start_ts = match NaiveDateTime::parse_from_str(format!("{}.01T00:00:00", month_str).as_str(), "%Y.%m.%dT%H:%M:%S"){
            Ok(t) => {t}
            Err(e) => {
                panic!("parsing {} failed: {}", month_str, e.to_string())
            }
        };
        let end_ts = start_ts + chrono::Duration::days(31);
        HashSet::from_iter(items
            .filter(collector_id.eq(collector))
            .filter(ts_start.ge(start_ts))
            .filter(ts_start.le(end_ts))
            .select(url).load::<String>(&self.conn).unwrap().into_iter())
    }

    pub fn get_urls_unverified(&self, limit: i64) -> Vec<Item> {
        use schema::items::dsl::*;
        items.filter(file_size.eq(0))
            .order(ts_start.desc())
            .limit(limit)
            .load::<Item>(&self.conn).unwrap()
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
        let _ = dotenv::dotenv();
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let conn = DbConnection::new(db_url.as_str());

        let collectors = vec![Collector{
            id: "rrc00".to_string(),
            project: "riperis".to_string(),
            url: "http://data.ris.ripe.net/rrc00".to_string()
        }];
        let mut entries = vec![];
        info!("creating entries...");
        for t in 1..1_000 {
            entries.push(Item{
                ts_start: t,
                ts_end: t+60*5,
                collector_id: "rrc00".to_string(),
                data_type: "rib".to_string(),
                url: format!("{}-test", t),
                file_size: 0,
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