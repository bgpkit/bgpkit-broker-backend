use clap::Parser;
use log::info;
use futures::StreamExt;
use bgpkit_broker_backend::config::Config;
use bgpkit_broker_backend::db::models::Collector;
use bgpkit_broker_backend::db::sqlite::BrokerDb;
use bgpkit_broker_backend::scrapers::{RipeRisScraper, RouteViewsScraper};

/// BGPKIT Broker data updater utility program
#[derive(Parser)]
#[clap(author, version)]
struct Opts {
    /// Collectors config file
    #[clap(short, long)]
    collectors_config: String,

    /// Index wanted to scrape from, default to scrape from all collectors
    #[clap(long)]
    collector_id: Option<String>,

    /// SQLite database path, default writes to bgpkit-broker-data.sqlite3
    #[clap(short, long)]
    db_path: Option<String>,

    /// Only scrape most recent data
    #[clap(short, long)]
    latest: bool,
}

async fn run_scraper(c: &Collector, latest:bool, db_path: &str) {
    match c.project.as_str() {
        "routeviews" => {
            RouteViewsScraper{update_mode: latest}.scrape(c, latest, db_path).await.unwrap();
        }
        "riperis" => {
            RipeRisScraper{update_mode: latest}.scrape(c, latest, db_path).await.unwrap();
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

    let db_path = match &opts.db_path {
        None => {"./bgpkit-broker-data.sqlite3".to_string()}
        Some(p) => {p.to_owned()}
    };

    let mut conn = BrokerDb::new(db_path.as_str());
    conn.insert_collectors(&collectors);
    conn.close();

    rt.block_on(async {

        let buffer_size = match opts.latest {
            true => 20,
            false => 1,
        };

        let mut stream = futures::stream:: iter(&collectors)
            .map(|c| run_scraper(c, opts.latest, db_path.as_str()))
            .buffer_unordered(buffer_size);

        info!("start scraping for {} collectors", &collectors.len());
        while let Some(_) = stream.next().await  { }
    });
}

