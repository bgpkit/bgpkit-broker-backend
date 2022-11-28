use std::env;
use clap::Parser;
use log::info;
use futures::StreamExt;
use bgpkit_broker_backend::config::Config;
use bgpkit_broker_backend::db::DbConnection;
use bgpkit_broker_backend::db::models::Collector;
use bgpkit_broker_backend::scrapers::{CrawlMode, RipeRisScraper, RouteViewsScraper};

#[derive(Parser)]
struct Opts {
    /// Collectors config file
    #[clap(short, long)]
    collectors_config: String,

    /// Database URL string, this overwrites the DATABASE_URL env variable
    #[clap(short, long)]
    db_url: Option<String>,

    /// Crawl mode: latest, two_months, bootstrap
    #[clap(short, long)]
    mode: CrawlMode,

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
    kafka_broker: Option<String>,

    /// Kafka topic for new file notification
    #[cfg(feature = "kafka")]
    #[clap(long)]
    kafka_topic: Option<String>,
}

async fn run_scraper(c: &Collector, mode: CrawlMode, conn: &DbConnection) {
    match c.project.as_str() {
        "routeviews" => {
            RouteViewsScraper{ mode }.scrape(c, Some(conn)).await.unwrap();
        }
        "riperis" => {
            RipeRisScraper{ mode }.scrape(c, Some(conn)).await.unwrap();
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
            // Database access string used by Broker API
            // DATABASE_URL=postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@$POSTGRES_HOST/$POSTGRES_DB

            match env::var("DATABASE_URL") {
                Ok(url) => {
                    // DATABASE_URL already set, use the one specified
                    url
                }
                Err(_) => {
                    let host = env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be set");
                    let password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
                    let user = env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
                    let db = env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");
                    format!("postgres://{}:{}@{}/{}", user, password, host, db)
                }
            }
        }
    };

    rt.block_on(async {
        #[cfg(not(feature="kafka"))]
            let conn = DbConnection::new(&db_url).await;
        #[cfg(feature="kafka")]
            let conn = DbConnection::new_with_kafka(&db_url, opts.kafka_broker.as_deref(), opts.kafka_topic.as_deref()).await;
        conn.insert_collectors(&collectors).await;

        let buffer_size = match &opts.mode {
            CrawlMode::Latest| CrawlMode::TwoMonths => {20}
            CrawlMode::Bootstrap => {1}
        };

        let mut stream = futures::stream:: iter(&collectors)
            .map(|c| run_scraper(c, opts.mode, &conn))
            .buffer_unordered(buffer_size);

        info!("start scraping for {} collectors", &collectors.len());
        while let Some(_) = stream.next().await  { }
    });
}

