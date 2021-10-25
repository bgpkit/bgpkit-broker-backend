pub mod schema;

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
    use super::*;

    #[test]
    fn test_insert() {
        env_logger::init();
        let conn = DbConnection::new("postgres://billboard_user:billboard@10.2.2.103/billboard");

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

}