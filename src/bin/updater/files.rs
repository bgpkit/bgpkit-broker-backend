use std::env;
use clap::Parser;
use log::info;
use futures::StreamExt;
use bgpkit_broker_backend::config::Config;
use bgpkit_broker_backend::db::DbConnection;
use bgpkit_broker_backend::db::models::Collector;
use bgpkit_broker_backend::scrapers::{RipeRisScraper, RouteViewsScraper};

#[derive(Parser)]
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

    /// Verify files available and get file sizes
    #[clap(short, long)]
    verify: bool,

    /// Index wanted to scrape from, default to scrape from all collectors
    #[clap(long)]
    collector_id: Option<String>,

    /// Kafka broker URL for new file notification
    #[cfg(feature = "kafka")]
    #[clap(long)]
    kafka_broker: String,

    /// Kafka topic for new file notification
    #[cfg(feature = "kafka")]
    #[clap(long)]
    kafka_topic: String,
}

async fn run_scraper(c: &Collector, latest:bool, conn: &DbConnection) {
    match c.project.as_str() {
        "routeviews" => {
            RouteViewsScraper{update_mode: latest}.scrape(c, latest, Some(conn)).await.unwrap();
        }
        "riperis" => {
            RipeRisScraper{update_mode: latest}.scrape(c, latest, Some(conn)).await.unwrap();
        }
        _ => {panic!("")}
    }
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
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all()
        .max_blocking_threads(blocking_cpus)
        .build().unwrap();

    let opts: Opts = Opts::parse();
    let config_file = std::fs::File::open(&opts.collectors_config).unwrap();
    let config:Config = serde_json::from_reader(config_file).unwrap();
    let collectors = config.to_collectors().into_iter()
        .filter(|c| {
            match &opts.collector_id{
                None => {true}
                Some(id) => {
                    id.as_str()==c.id
                }
            }
        }).collect::<Vec<Collector>>();

    let db_url = match opts.db_url.clone() {
        Some(url) => url,
        None => {
            env::var("DATABASE_URL").expect("DATABASE_URL must be set")
        }
    };


    rt.block_on(async {
        #[cfg(not(feature="kafka"))]
            let conn = DbConnection::new(&db_url).await;
        #[cfg(feature="kafka")]
            let conn = DbConnection::new_with_kafka(&db_url, opts.kafka_broker.as_str(), opts.kafka_topic.as_str()).await;
        conn.insert_collectors(&collectors).await;

        let buffer_size = match opts.latest {
            true => 20,
            false => 1,
        };

        let mut stream = futures::stream:: iter(&collectors)
            .map(|c| run_scraper(c, opts.latest, &conn))
            .buffer_unordered(buffer_size);

        info!("start scraping for {} collectors", &collectors.len());
        while let Some(_) = stream.next().await  { }
    });
}

