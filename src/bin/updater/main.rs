use std::env;
use clap::Clap;
use log::info;
use futures::future::join_all;
use dotenv;
use bgpkit_broker_backend::config::Config;
use bgpkit_broker_backend::db::DbConnection;
use bgpkit_broker_backend::kafka::KafkaProducer;
use bgpkit_broker_backend::models::Collector;
use bgpkit_broker_backend::scrapers::{RipeRisScraper, RouteViewsScraper};

#[derive(Clap)]
struct Opts {
    /// Collectors config file
    #[clap(short, long)]
    collectors_config: String,

    /// Database URL string, this overwrites the DATABASE_URL env variable
    #[clap(short, long)]
    db_url: Option<String>,

    /// Only scrape most recent data
    #[clap(short, long)]
    latest: bool,

    /// Pretty print
    #[clap(short, long)]
    pretty: bool,

    /// Index wanted to scrape from, default to scrape from all collectors
    #[clap(long)]
    collector_id: Option<String>,

    /// Kafka broker URL
    #[clap(long)]
    kafka_broker: Option<String>,

    /// Kafka topic
    #[clap(long)]
    kafka_topic: Option<String>,
}

fn main () {
    // init logger
    env_logger::init();

    let _ = dotenv::dotenv();

    // configure async runtime
    let blocking_cpus = match num_cpus::get() {
        1 => 1,
        n => n/2,
    };
    info!("using {} cores for parsing html pages", blocking_cpus);
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().max_blocking_threads(blocking_cpus).build().unwrap();

    let opts: Opts = Opts::parse();
    let config_file = std::fs::File::open(&opts.collectors_config).unwrap();
    let config:Config = serde_json::from_reader(config_file).unwrap();
    let collectors = config.to_collectors().into_iter()
        .filter_map(|c| {
            match &opts.collector_id{
                None => {Some(c)}
                Some(id) => {
                    if id.as_str()==c.id { Some(c)}
                    else {None}
                }
            }
        }).collect::<Vec<Collector>>();

    let db_url = match opts.db_url.clone() {
        Some(url) => url,
        None => {
            env::var("DATABASE_URL").expect("DATABASE_URL must be set")
        }
    };

    let conn = DbConnection::new(&db_url);
    conn.insert_collectors(&collectors);

    let kafka: Box<KafkaProducer>;
    let kafka_producer = match &opts.kafka_broker {
        None => { None }
        Some(b) => {
            let topic = &opts.kafka_topic.clone().unwrap_or("broker-new-items".to_string());
            kafka = Box::new(KafkaProducer::new(  b.as_str(),  topic));
            Some(kafka.as_ref())
        }
    };

    rt.block_on(async {
        let rv_scraper = RouteViewsScraper{};
        let ris_scraper = RipeRisScraper{};

        let mut rv_futures = vec![];
        let mut ris_futures = vec![];

        collectors.iter().for_each(|c| {
            match c.project.as_str() {
                "routeviews" => {
                    rv_futures.push(rv_scraper.scrape(c, opts.latest.clone(), Some(&conn), kafka_producer));
                }
                "riperis" => {
                    ris_futures.push(ris_scraper.scrape(c, opts.latest.clone(), Some(&conn), kafka_producer));
                }
                _ => {panic!("")}
            }
        });

        info!("start scraping for {} collectors", &collectors.len());

        join_all(rv_futures).await;
        join_all(ris_futures).await;
    });
}

