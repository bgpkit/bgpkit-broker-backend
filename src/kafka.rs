use std::time::Duration;
use rdkafka::{ClientConfig, Message, ClientContext};
use rdkafka::message::{Headers, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};
use log::{info, warn};
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, StreamConsumer};
use crate::models::Item;

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
                        .key(&item.ts_start.to_string())
                        .headers(OwnedHeaders::new().add("header_key", "header_value")),
                    Duration::from_secs(0),
                )
                .await;
        }
    }
}

// A simple context to customize the consumer behavior and print a log line every time
// offsets are committed
struct LoggingConsumerContext;

impl ClientContext for LoggingConsumerContext {}
impl ConsumerContext for LoggingConsumerContext {}

// Define a new type for convenience
type LoggingConsumer = StreamConsumer<LoggingConsumerContext>;

#[allow(unused)]
async fn consume_and_print(brokers: &str, group_id: &str, topics: &[&str]) {
    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .create_with_context(LoggingConsumerContext).unwrap();

    consumer
        .subscribe(&topics.to_vec())
        .expect("Can't subscribe to specified topics");

    loop {
        match consumer.recv().await {
            Err(e) => warn!("Kafka error: {}", e),
            Ok(m) => {
                let payload = match m.payload_view::<str>() {
                    None => None,
                    Some(Ok(s)) => Some(s),
                    Some(Err(e)) => {
                        warn!("Error while deserializing message payload: {:?}", e);
                        None
                    }
                };
                if let Some(p) = payload {
                    let item = serde_json::from_str::<Item>(p).unwrap();
                    info!("{:?}", item);
                }
                info!("key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                      m.key(), payload.unwrap(), m.topic(), m.partition(), m.offset(), m.timestamp());
                if let Some(headers) = m.headers() {
                    for i in 0..headers.count() {
                        let header = headers.get(i).unwrap();
                        info!("  Header {:#?}: {:?}", header.0, header.1);
                    }
                }
                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        };
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_producer() {
        env_logger::init();
        let items = vec![
            Item {
                ts_start: 0,
                ts_end: 0,
                collector_id: "rro00".to_string(),
                data_type: "rib".to_string(),
                url: "http://testurl.com".to_string(),
                file_size: 0,
            },
            Item {
                ts_start: 0,
                ts_end: 0,
                collector_id: "rro00".to_string(),
                data_type: "rib".to_string(),
                url: "http://testurl.com".to_string(),
                file_size: 0,
            },
        ];

        let producer = KafkaProducer::new("10.2.2.107:9092", "test");
        producer.produce(&items).await;
    }
    #[tokio::test]
    async fn test_listener() {
        env_logger::init();
        consume_and_print("10.2.2.107:9092", "consume1", &["test"]).await
    }
}