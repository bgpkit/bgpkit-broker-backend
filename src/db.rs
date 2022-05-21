use chrono::{NaiveDateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Collector {
    pub id: String,
    pub project: String,
    pub url: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item {
    pub ts_start: chrono::NaiveDateTime,
    pub ts_end: chrono::NaiveDateTime,
    pub collector_id: String,
    pub data_type: String,
    pub url: String,
    pub rough_size: i64,
    pub exact_size: i64,
}

pub struct BrokerDb {
    conn: Connection
}

pub struct LogDb {
    conn: Connection
}

impl BrokerDb {
    pub fn new(path: &str) -> BrokerDb {
        let db = Connection::open(path).unwrap();
        db.execute(r#"
            CREATE TABLE IF NOT EXISTS "items"(
              "ts_start" INTEGER,
              "ts_end" INTEGER,
              "collector_id" TEXT,
              "data_type" TEXT,
              "url" TEXT,
              "rough_size" TEXT,
              "exact_size" TEXT,
              UNIQUE(ts_start, collector_id, data_type)
            );
        "#,
                   [],
        ).unwrap();
        db.execute(r#"
            CREATE TABLE IF NOT EXISTS "collectors"(
              "id" TEXT,
              "project" TEXT,
              "url" TEXT
            );
        "#, []).unwrap();
        db.execute(r#"
            CREATE INDEX IF NOT EXISTS ts_index ON "items" (ts_start DESC, ts_end ASC);
        "#, []).unwrap();
        db.execute(r#"
            CREATE INDEX IF NOT EXISTS grouping ON "items" (collector_id, data_type);
        "#, []).unwrap();
        BrokerDb {conn: db}
    }

    pub fn clear(&self) {
        self.conn.execute(
            "delete from items;",
            []
        ).unwrap();
        self.conn.execute(
            "delete from collectors;",
            []
        ).unwrap();
    }

    pub fn count_records_in_month(&self, collector: &str, month_str: &str) -> i64 {
        let start_ts = match NaiveDateTime::parse_from_str(format!("{}.01T00:00:00", month_str).as_str(), "%Y.%m.%dT%H:%M:%S"){
            Ok(t) => {t}
            Err(e) => {
                panic!("parsing {} failed: {}", month_str, e)
            }
        };
        let end_ts = start_ts + chrono::Duration::days(31);
        let sql = format!(r#"
        select count(*) from items
        where collector_id = '{}' and ts_start >= {} and ts_start < {}"#,
                          collector, start_ts.timestamp(), end_ts.timestamp());
        self.conn.query_row(sql.as_str(), [], |row| row.get(0)).unwrap()
    }

    pub fn insert_items(&mut self, items: &[Item]) -> usize {
        let tx = self.conn.transaction().unwrap();
        let mut total_inserted = 0;
        for item in items {
            let ts_start = item.ts_start.timestamp();
            let ts_end = item.ts_end.timestamp();
            let collector_id = item.collector_id.as_str();
            let data_type = item.data_type.as_str();
            let url = item.url.as_str();
            let rough_size = item.rough_size;
            let exact_size = item.exact_size;
            total_inserted += tx.execute(
                "INSERT OR IGNORE INTO items \
                (ts_start, ts_end, collector_id, data_type, url, rough_size, exact_size) \
                values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    ts_start, ts_end, collector_id, data_type, url, rough_size, exact_size
                ]
            ).unwrap();
        }
        tx.commit().unwrap();
        total_inserted
    }

    pub fn insert_collectors(&mut self, collectors: &[Collector]) {
        let tx = self.conn.transaction().unwrap();
        for collector in  collectors {
            tx.execute(
                "INSERT OR IGNORE INTO collectors (id, project, url) values (?1, ?2, ?3)",
                params![collector.id, collector.project, collector.url]
            ).unwrap();
        }
        tx.commit().unwrap();
    }

    pub fn get_collectors(&self) -> Vec<Collector> {
        let mut stmt = self.conn.prepare("SELECT id, project, url FROM collectors").unwrap();
        let person_iter = stmt.query_map([], |row| {
            Ok(Collector {
                id: row.get(0).unwrap(),
                project: row.get(1).unwrap(),
                url: row.get(2).unwrap(),
            })
        }).unwrap();

        person_iter.filter_map(|v| v.ok()).collect::<Vec<Collector>>()
    }

    pub fn count_items(&self) -> i64 {
        self.conn.query_row("select count(*) from items", [], |row| row.get(0)).unwrap()
    }

    pub fn close(self) {
        let _ = self.conn.close();
    }

}

impl LogDb {
    pub fn new(path: &str) -> LogDb {
        let db = Connection::open(path).unwrap();
        db.execute(r#"
            CREATE TABLE IF NOT EXISTS "logs"(
              "time" TEXT,
              "type" TEXT,
              "message" TEXT,
            );
        "#,
                   [],
        ).unwrap();
        LogDb { conn: db }
    }

    pub fn log(&self, msg_type: &str, msg_content: &str) {
        self.conn.execute("insert into logs (time, type, message) values (?1, ?2, ?3)", params![Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S").to_string(), msg_type.to_string(), msg_content.to_string()]).unwrap();
    }

    pub fn close(self) {
        self.conn.close().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use super::*;

    #[test]
    fn test_db_creation() {
        let mut db = BrokerDb::new("test-db.sqlite3");
        db.clear();
        db.insert_collectors(&[Collector{
            id: "test".to_string(),
            project: "tp".to_string(),
            url: "example.com".to_string()
        }]);

        db.insert_items(&[Item{
            ts_start: NaiveDateTime::parse_from_str("2022-02-01T00:00:00","%Y-%m-%dT%H:%M:%S").unwrap(),
            ts_end: NaiveDateTime::parse_from_str("2022-02-02T00:00:00","%Y-%m-%dT%H:%M:%S").unwrap(),
            collector_id: "test".to_string(),
            data_type: "rib".to_string(),
            url: "example.com/rib-1.gz".to_string(),
            rough_size: 100,
            exact_size: 0
        }]);

        let collectors = db.get_collectors();
        assert_eq!(1, collectors.len());
        let c = collectors.get(0).unwrap();
        assert_eq!(c.id.as_str(), "test");
        assert_eq!(c.project.as_str(), "tp");
        assert_eq!(c.url.as_str(), "example.com");
        db.close();
    }

    #[test]
    fn test_unique() {
        let mut db = BrokerDb::new("test-db.sqlite3");
        db.clear();
        let item = Item{
            ts_start: NaiveDateTime::parse_from_str("2022-02-01T00:00:00","%Y-%m-%dT%H:%M:%S").unwrap(),
            ts_end: NaiveDateTime::parse_from_str("2022-02-02T00:00:00","%Y-%m-%dT%H:%M:%S").unwrap(),
            collector_id: "test".to_string(),
            data_type: "rib".to_string(),
            url: "example.com/rib-1.gz".to_string(),
            rough_size: 100,
            exact_size: 0
        };
        db.insert_items(&[item.clone(), item]);
        assert_eq!(db.count_items(), 1);
        db.close();
    }

    #[test]
    fn test_count_records_in_month() {
        let mut db = BrokerDb::new("test-db.sqlite3");
        db.clear();
        let item = Item{
            ts_start: NaiveDateTime::parse_from_str("2022-02-01T00:00:00","%Y-%m-%dT%H:%M:%S").unwrap(),
            ts_end: NaiveDateTime::parse_from_str("2022-02-01T00:05:00","%Y-%m-%dT%H:%M:%S").unwrap(),
            collector_id: "test".to_string(),
            data_type: "rib".to_string(),
            url: "example.com/rib-1.gz".to_string(),
            rough_size: 100,
            exact_size: 0
        };
        db.insert_items(&[item.clone(), item]);
        assert_eq!(db.count_records_in_month("test", "2022.02"), 1);
        assert_eq!(db.count_records_in_month("test", "2022.01"), 0);
    }
}
