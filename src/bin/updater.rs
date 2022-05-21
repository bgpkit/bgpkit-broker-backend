use std::fs::File;
use clap::Parser;
use log::info;
use futures::StreamExt;
use bgpkit_broker_backend::config::Config;
use bgpkit_broker_backend::db::*;
use bgpkit_broker_backend::scrapers::{RipeRisScraper, RouteViewsScraper};

/// BGPKIT Broker data updater utility program
#[derive(Parser)]
#[clap(author, version)]
struct Opts {
    /// Path to the collectors config file that specifies BGP collectors and the data root URLs.
    /// By default pulling data from https://data.bgpkit.com/broker/collectors-full-config.json
    #[clap(short('C'), long)]
    collectors_config: Option<String>,

    /// Collector ID to run the crawler against, e.g. rrc00 or route-views2
    #[clap(short, long)]
    collector_id: Option<String>,

    /// Path to the Broker SQLite database, default writes to "./bgpkit-broker-data.sqlite3"
    #[clap(short, long)]
    db_path: Option<String>,

    /// Flag set to scrape only the most recent month's of data
    #[clap(short, long)]
    latest: bool,

    /// Flag set to bootstrap the initial database from
    /// https://data.bgpkit.com/broker/bgpkit-broker-data.sqlite3
    #[clap(short, long)]
    bootstrap: bool,
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

    let config: Config =
    if let Some(config_path) = &opts.collectors_config {
        let config_file = std::fs::File::open(config_path.as_str()).unwrap();
        serde_json::from_reader(config_file).unwrap()
    } else {
        reqwest::blocking::get("https://data.bgpkit.com/broker/collectors-full-config.json").unwrap().json().unwrap()
    };

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

    if opts.bootstrap {
        let mut file = File::create(db_path.as_str()).unwrap();
        let bootstrap_url = "https://data.bgpkit.com/broker/bgpkit-broker-data.sqlite3";
        info!("start downloading file {}...", bootstrap_url);
        let file_len = reqwest::blocking::get(bootstrap_url).unwrap()
            .copy_to(&mut file).unwrap();
        info!("start downloading file {}... downloaded with size {}", bootstrap_url, file_len);
    }

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
        while stream.next().await.is_some()  { }
    });
}

