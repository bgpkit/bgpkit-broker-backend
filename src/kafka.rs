use std::time::Duration;
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use log::info;
use crate::db::models::Item;

pub struct KafkaProducer {
    producer: FutureProducer,
    topic: String
}

impl KafkaProducer {
    pub fn new(brokers: &str, topic_name: &str) -> KafkaProducer {
        info!("initializing kafka producer with broker: {} , topic: {}", brokers, topic_name);
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create().unwrap();

        KafkaProducer{
            producer,
            topic: topic_name.to_string()
        }
    }

    pub(crate) async fn produce(&self, items: &Vec<Item>) {
        for item in items {
            let payload = serde_json::to_string(item).unwrap();
            let _ = self.producer
                .send(
                    FutureRecord::to(&self.topic)
                        .payload(&payload)
                        .key(&item.ts_start.to_string()),
                        // .headers(OwnedHeaders::new().add("header_key", "header_value")),
                    Duration::from_secs(0),
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use super::*;

    #[tokio::test]
    async fn test_producer() {
        env_logger::init();
        let items = vec![
            Item {
                ts_start: NaiveDateTime::from_timestamp(1658514053, 0),
                ts_end: NaiveDateTime::from_timestamp(1658514054, 0),
                collector_id: "rrc00".to_string(),
                data_type: "rib".to_string(),
                url: "http://testurl.com".to_string(),
                rough_size: 0,
                exact_size: 0
            },
            Item {
                ts_start: NaiveDateTime::from_timestamp(1658514053, 0),
                ts_end: NaiveDateTime::from_timestamp(1658514054, 0),
                collector_id: "rrc01".to_string(),
                data_type: "rib".to_string(),
                url: "http://testurl.com".to_string(),
                rough_size: 0,
                exact_size: 0
            },
        ];

        let producer = KafkaProducer::new("127.0.0.1:9092", "test-kafka");
        producer.produce(&items).await;
    }
}