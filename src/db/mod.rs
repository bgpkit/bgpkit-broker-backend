pub mod models;
pub mod kafka;

use std::collections::HashSet;
use std::iter::FromIterator;
use chrono::NaiveDateTime;

use log::info;
use sqlx::{Executor, PgPool, Postgres, QueryBuilder, Row};
use sqlx::postgres::{PgConnectOptions, PgRow};

use crate::db::models::{Collector, Item};

#[cfg(feature = "kafka")]
use crate::db::kafka::KafkaProducer;

const CHUNK_SIZE: usize = 60_000;


pub struct DbConnection {
    pub pool: PgPool,

    #[cfg(feature = "kafka")]
    pub kafka: Option<KafkaProducer>,
}


fn url_to_options(db_url: &str, disable_prepare: bool) -> PgConnectOptions {
    let parsed = url::Url::parse(db_url).unwrap();
    let mut opts = PgConnectOptions::new()
        .host(parsed.host().unwrap().to_string().as_str());
    if parsed.username()!="" {
        opts = opts.username(parsed.username());
    }

    if let Some(password) = parsed.password() {
        opts = opts.password(password);
    }

    if let Some(port) = parsed.port() {
        opts = opts.port(port);
    }

    let parts = db_url.split("/").collect::<Vec<&str>>();
    let db_name = parts.into_iter().last().unwrap();
    opts = opts.database(db_name);

    if disable_prepare {
        opts = opts.statement_cache_capacity(0);
    }

    opts
}

impl DbConnection {
    #[cfg(feature = "kafka")]
    pub async fn new(db_url: &str) -> DbConnection {
        let options = url_to_options(db_url, true);
        let pool = PgPool::connect_with(options).await.unwrap();
        DbConnection{ pool, kafka: None }
    }

    #[cfg(not(feature = "kafka"))]
    pub async fn new(db_url: &str) -> DbConnection {
        let options = url_to_options(db_url, true);
        let pool = PgPool::connect_with(options).await.unwrap();
        DbConnection{ pool }
    }

    #[cfg(feature="kafka")]
    pub async fn new_with_kafka(db_url: &str, kafka_brokers: &str, kafka_topic: &str) -> DbConnection {
        let options = url_to_options(db_url, true);
        let pool = PgPool::connect_with(options).await.unwrap();
        let kafka = Some(KafkaProducer::new(kafka_brokers, kafka_topic));
        DbConnection{ pool, kafka }
    }

    pub async fn insert_collectors(&self, entries: &Vec<Collector>){
        info!("inserting collectors info");

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO collectors(id, project, url) "
        );
        query_builder.push_values(entries, |mut b, collector| {
            b.push_bind(collector.id.as_str())
                .push_bind(collector.project.as_str())
                .push_bind(collector.url.as_str());
        });
        query_builder.push(
            " ON CONFLICT DO NOTHING "
        );
        let query = query_builder.build();
        let _res = self.pool.execute(query).await;
    }

    pub async fn count_records_in_month(&self, collector: &str, month_str: &str) -> i64 {

        let start_ts = match NaiveDateTime::parse_from_str(format!("{}.01T00:00:00", month_str).as_str(), "%Y.%m.%dT%H:%M:%S"){
            Ok(t) => {t}
            Err(e) => {
                panic!("parsing {} failed: {}", month_str, e.to_string())
            }
        };
        let end_ts = start_ts + chrono::Duration::days(31);

        let records = sqlx::query(
           r#"
           SELECT count(*) as c
           FROM items
           WHERE collector_id=$1 AND
           ts_start >= $2 AND
           ts_start <= $3
           "#,
       )
            .bind(collector)
            .bind(&start_ts)
            .bind(&end_ts)
            .fetch_one(&self.pool)
            .await.unwrap();

        records.try_get("c").unwrap()
    }

    pub async fn get_urls_in_month(&self, collector: &str, month_str: &str) -> HashSet<String> {
        let start_ts = match NaiveDateTime::parse_from_str(format!("{}.01T00:00:00", month_str).as_str(), "%Y.%m.%dT%H:%M:%S"){
            Ok(t) => {t}
            Err(e) => {
                panic!("parsing {} failed: {}", month_str, e.to_string())
            }
        };
        let end_ts = start_ts + chrono::Duration::days(31);
        let urls = sqlx::query(
            r#"
           SELECT url
           FROM items
           WHERE collector_id=$1 AND
           ts_start >= $2 AND
           ts_start <= $3
           "#,
        )
            .bind(collector)
            .bind(&start_ts)
            .bind(&end_ts)
            .fetch_all(&self.pool).await.unwrap()
            .iter().map(|r|r.get::<String,_>("url").to_string()).collect::<Vec<String>>();

        HashSet::from_iter(urls.into_iter())
    }

    pub async fn insert_items(&self, entries: &Vec<Item>) -> Vec<Item> {
        let mut inserted = vec![];
        for chunk in entries.chunks(CHUNK_SIZE/7){
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO items(ts_start, ts_end, collector_id, data_type, url, rough_size, exact_size) "
            );
            query_builder.push_values(chunk, |mut b, item| {
                b
                    .push_bind(&item.ts_start)
                    .push_bind(&item.ts_end)
                    .push_bind(item.collector_id.as_str())
                    .push_bind(item.data_type.as_str())
                    .push_bind(item.url.as_str())
                    .push_bind(item.rough_size)
                    .push_bind(item.exact_size)
                ;
            });
            query_builder.push(
                " ON CONFLICT DO NOTHING "
            );
            query_builder.push(
            " RETURNING *"
            );
            let query = query_builder.build();
            let res: Vec<Item> = query.fetch_all(&self.pool).await.unwrap().into_iter().map(|row: PgRow|{
                Item{
                    ts_start: row.try_get("ts_start").unwrap(),
                    ts_end: row.try_get("ts_end").unwrap(),
                    collector_id: row.try_get("collector_id").unwrap(),
                    data_type: row.try_get("data_type").unwrap(),
                    url: row.try_get("url").unwrap(),
                    rough_size: row.try_get("rough_size").unwrap(),
                    exact_size: row.try_get("exact_size").unwrap()
                }
            }).collect();
            inserted.extend(res);
        }
        inserted
    }

    #[cfg(feature="kafka")]
    pub async fn notify(&self, items: &Vec<Item>) {
        if let Some(kafka) = &self.kafka {
            kafka.produce(items).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert() {
        let db = DbConnection::new("postgres://localhost/mingwei").await;

        let collectors = vec![
            Collector{
                id: "rrc00".to_string(),
                project: "ris".to_string(),
                url: "1".to_string()
            },
            Collector{
                id: "rrc01".to_string(),
                project: "ris".to_string(),
                url: "2".to_string()
            },
        ];
        db.insert_collectors(&collectors).await;

        let items = vec![
            Item{
                ts_start: chrono::Utc::now().naive_utc(),
                ts_end: chrono::Utc::now().naive_utc(),
                collector_id: "rrc00".to_string(),
                data_type: "update".to_string(),
                url: "test".to_string(),
                rough_size: 0,
                exact_size: 1
            },
            Item{
                ts_start: chrono::Utc::now().naive_utc(),
                ts_end: chrono::Utc::now().naive_utc(),
                collector_id: "rrc00".to_string(),
                data_type: "update".to_string(),
                url: "test2".to_string(),
                rough_size: 0,
                exact_size: 2
            },
        ];
        let inserted = db.insert_items(&items).await;
        assert_eq!(inserted.len(), 2);
        let inserted = db.insert_items(&items).await;
        assert_eq!(inserted.len(), 0);

        dbg!(db.get_urls_in_month("rrc00", "2022.08").await);
        dbg!(db.count_records_in_month("rrc00", "2022.08").await);
    }
}